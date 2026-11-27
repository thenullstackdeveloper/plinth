//! What an image file really is, read from its header: the format, the
//! size its header declares, and whether it is animated. Nothing is decoded:
//! the header is enough to refuse what a template may not use, and the
//! webview decodes the rest.

use plinth_domain::ImageKind;

/// What the header of an image says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    pub kind: ImageKind,
    pub width: u32,
    pub height: u32,
    /// Whether the file holds an animation. `inspect` refuses animated
    /// images, so an `ImageInfo` it returns always says `false`.
    pub animated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    /// The bytes are not in any format Plinth reads.
    #[error("not an image")]
    NotAnImage,
    /// The bytes start like a known format but break its rules.
    #[error("{0}")]
    Malformed(&'static str),
}

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Reads the format and the size of an image from its header.
///
/// A PNG must start with `IHDR`, declare a size other than zero and carry
/// no `acTL` chunk (an animated PNG).
///
/// A WebP is a `RIFF` whose size is that of the file less 8, of form
/// `WEBP`, whose first chunk is `VP8 ` (lossy), `VP8L` (lossless) or `VP8X`
/// (extended). An extended file with its animation flag, or any file with
/// an `ANIM` or `ANMF` chunk, is animated.
pub fn inspect(bytes: &[u8]) -> Result<ImageInfo, ImageError> {
    if let Some(chunks) = bytes.strip_prefix(PNG_SIGNATURE) {
        return inspect_png(chunks);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return inspect_webp(bytes);
    }
    Err(ImageError::NotAnImage)
}

/// One PNG chunk: its type and its data, without the length and the CRC.
struct Chunk<'a> {
    kind: &'a [u8],
    data: &'a [u8],
}

/// The chunks after the signature, until the bytes end or one is cut short.
fn png_chunks(mut rest: &[u8]) -> impl Iterator<Item = Result<Chunk<'_>, ImageError>> {
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let truncated = Err(ImageError::Malformed("truncated"));
        let Some((head, tail)) = rest.split_at_checked(8) else {
            rest = &[];
            return Some(truncated);
        };
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
        let len = usize::try_from(len).unwrap_or(usize::MAX);
        // The data and its 4-byte CRC.
        let Some((data, after)) = tail
            .split_at_checked(len)
            .and_then(|(data, after)| Some((data, after.get(4..)?)))
        else {
            rest = &[];
            return Some(truncated);
        };
        rest = after;
        Some(Ok(Chunk {
            kind: &head[4..8],
            data,
        }))
    })
}

fn inspect_png(chunks: &[u8]) -> Result<ImageInfo, ImageError> {
    let mut chunks = png_chunks(chunks);
    let first = chunks
        .next()
        .unwrap_or(Err(ImageError::Malformed("truncated")))?;
    if first.kind != b"IHDR" || first.data.len() != 13 {
        return Err(ImageError::Malformed("the first chunk is not IHDR"));
    }
    let width = u32::from_be_bytes([first.data[0], first.data[1], first.data[2], first.data[3]]);
    let height = u32::from_be_bytes([first.data[4], first.data[5], first.data[6], first.data[7]]);
    if width == 0 || height == 0 {
        return Err(ImageError::Malformed("zero size"));
    }
    // An APNG declares its animation in `acTL`, before the first `IDAT`.
    for chunk in chunks {
        let chunk = chunk?;
        match chunk.kind {
            b"acTL" => return Err(ImageError::Malformed("animated")),
            b"IDAT" | b"IEND" => break,
            _ => {}
        }
    }
    Ok(ImageInfo {
        kind: ImageKind::Png,
        width,
        height,
        animated: false,
    })
}

/// The `VP8X` flag of an animated file.
const VP8X_ANIMATION: u8 = 0x02;

fn le16(bytes: &[u8]) -> u32 {
    u32::from(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn le24(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0])
}

/// The chunks of a WebP after `WEBP`, each padded to an even length, until
/// the bytes end or one is cut short.
fn webp_chunks(mut rest: &[u8]) -> impl Iterator<Item = Result<Chunk<'_>, ImageError>> {
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let truncated = Err(ImageError::Malformed("truncated"));
        let Some((head, tail)) = rest.split_at_checked(8) else {
            rest = &[];
            return Some(truncated);
        };
        let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
        let len = usize::try_from(len).unwrap_or(usize::MAX);
        let Some((data, after)) = tail.split_at_checked(len) else {
            rest = &[];
            return Some(truncated);
        };
        // An odd chunk is followed by one byte of padding.
        rest = if len % 2 == 1 {
            after.get(1..).unwrap_or(&[])
        } else {
            after
        };
        Some(Ok(Chunk {
            kind: &head[..4],
            data,
        }))
    })
}

fn inspect_webp(bytes: &[u8]) -> Result<ImageInfo, ImageError> {
    let declared = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if usize::try_from(declared).ok() != Some(bytes.len() - 8) {
        return Err(ImageError::Malformed(
            "the RIFF size does not match the file",
        ));
    }
    let mut chunks = webp_chunks(&bytes[12..]);
    let first = chunks
        .next()
        .unwrap_or(Err(ImageError::Malformed("truncated")))?;
    let truncated = ImageError::Malformed("truncated");
    let (width, height) = match first.kind {
        b"VP8 " => {
            // A frame tag of 3 bytes, the start code, then the size in the
            // low 14 bits of each of two 16-bit numbers.
            let data = first.data.get(..10).ok_or(truncated)?;
            if data[0] & 1 != 0 || data[3..6] != [0x9d, 0x01, 0x2a] {
                return Err(ImageError::Malformed("the VP8 frame is not a key frame"));
            }
            (le16(&data[6..8]) & 0x3fff, le16(&data[8..10]) & 0x3fff)
        }
        b"VP8L" => {
            // The signature, then the size less one in 14 bits each.
            let data = first.data.get(..5).ok_or(truncated)?;
            if data[0] != 0x2f {
                return Err(ImageError::Malformed("the VP8L signature is missing"));
            }
            let bits = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
            ((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1)
        }
        b"VP8X" => {
            // The flags, three reserved bytes, then the size less one in 24
            // bits each.
            let data = first.data.get(..10).ok_or(truncated)?;
            if data[0] & VP8X_ANIMATION != 0 {
                return Err(ImageError::Malformed("animated"));
            }
            (le24(&data[4..7]) + 1, le24(&data[7..10]) + 1)
        }
        _ => {
            return Err(ImageError::Malformed(
                "the first chunk is not VP8, VP8L or VP8X",
            ))
        }
    };
    if width == 0 || height == 0 {
        return Err(ImageError::Malformed("zero size"));
    }
    for chunk in chunks {
        if matches!(chunk?.kind, b"ANIM" | b"ANMF") {
            return Err(ImageError::Malformed("animated"));
        }
    }
    Ok(ImageInfo {
        kind: ImageKind::Webp,
        width,
        height,
        animated: false,
    })
}

/// Small, valid images built byte by byte, for the tests of this crate.
#[cfg(test)]
pub(crate) mod test_images {
    /// CRC-32 (ISO-HDLC), as PNG uses it.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn adler32(bytes: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in bytes {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }

    /// A PNG chunk: length, type, data and CRC.
    pub fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut crc_input = kind.to_vec();
        crc_input.extend_from_slice(data);
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
        out
    }

    pub fn ihdr(width: u32, height: u32) -> Vec<u8> {
        let mut data = width.to_be_bytes().to_vec();
        data.extend_from_slice(&height.to_be_bytes());
        // 8-bit RGBA, deflate, no filter method, no interlace.
        data.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(b"IHDR", &data)
    }

    /// The pixels of a `width` × `height` RGBA image, all opaque red, as a
    /// zlib stream of stored blocks.
    pub fn idat(width: u32, height: u32) -> Vec<u8> {
        let row: Vec<u8> = std::iter::once(0u8)
            .chain((0..width).flat_map(|_| [200u8, 40, 40, 255]))
            .collect();
        let raw: Vec<u8> = (0..height).flat_map(|_| row.clone()).collect();
        let mut zlib = vec![0x78, 0x01];
        let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
        for (i, block) in blocks.iter().enumerate() {
            zlib.push(u8::from(i + 1 == blocks.len()));
            let len = u16::try_from(block.len()).unwrap();
            zlib.extend_from_slice(&len.to_le_bytes());
            zlib.extend_from_slice(&(!len).to_le_bytes());
            zlib.extend_from_slice(block);
        }
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        chunk(b"IDAT", &zlib)
    }

    /// The signature followed by `chunks`, as they are.
    pub fn png_from(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = super::PNG_SIGNATURE.to_vec();
        for c in chunks {
            out.extend_from_slice(c);
        }
        out
    }

    /// A valid `width` × `height` PNG.
    pub fn png(width: u32, height: u32) -> Vec<u8> {
        png_from(&[
            ihdr(width, height),
            idat(width, height),
            chunk(b"IEND", &[]),
        ])
    }

    /// A WebP of `chunks` (type and data), each padded to an even length,
    /// inside a `RIFF` whose size is that of the file less 8.
    pub fn webp(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut body = b"WEBP".to_vec();
        for (kind, data) in chunks {
            body.extend_from_slice(*kind);
            body.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
            body.extend_from_slice(data);
            if data.len() % 2 == 1 {
                body.push(0);
            }
        }
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    /// A `VP8L` chunk's data: the signature and the size, less one, in 14
    /// bits each, with the alpha bit set so a missing mask shows.
    pub fn vp8l(width: u32, height: u32) -> Vec<u8> {
        let bits = (width - 1) | ((height - 1) << 14) | (1 << 28);
        let mut data = vec![0x2f];
        data.extend_from_slice(&bits.to_le_bytes());
        data
    }

    /// A `VP8X` chunk's data: `flags`, three reserved bytes, and the size,
    /// less one, in 24 bits each.
    pub fn vp8x(flags: u8, width: u32, height: u32) -> Vec<u8> {
        let mut data = vec![flags, 0, 0, 0];
        data.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
        data.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
        data
    }

    /// A still lossless `width` × `height` WebP.
    pub fn webp_image(width: u32, height: u32) -> Vec<u8> {
        webp(&[(b"VP8L", vp8l(width, height))])
    }
}

#[cfg(test)]
mod tests {
    use super::test_images::{chunk, idat, ihdr, png, png_from, vp8l, vp8x, webp, webp_image};
    use super::*;

    #[test]
    fn inspect_reads_png_dimensions() {
        assert_eq!(
            inspect(&png(3, 2)),
            Ok(ImageInfo {
                kind: ImageKind::Png,
                width: 3,
                height: 2,
                animated: false,
            })
        );
    }

    #[test]
    fn inspect_rejects_an_apng() {
        // `acTL`: two frames, played forever.
        let actl = chunk(b"acTL", &[0, 0, 0, 2, 0, 0, 0, 0]);
        let apng = png_from(&[ihdr(3, 2), actl, idat(3, 2), chunk(b"IEND", &[])]);
        assert_eq!(inspect(&apng), Err(ImageError::Malformed("animated")));
    }

    #[test]
    fn inspect_rejects_a_png_whose_first_chunk_is_not_ihdr() {
        let text = chunk(b"tEXt", b"Comment\0first");
        let bytes = png_from(&[text, ihdr(3, 2), idat(3, 2), chunk(b"IEND", &[])]);
        assert_eq!(
            inspect(&bytes),
            Err(ImageError::Malformed("the first chunk is not IHDR"))
        );
    }

    #[test]
    fn inspect_rejects_zero_size() {
        for (width, height) in [(0, 2), (3, 0)] {
            assert_eq!(
                inspect(&png_from(&[ihdr(width, height), chunk(b"IEND", &[])])),
                Err(ImageError::Malformed("zero size")),
                "{width}x{height}"
            );
        }
        // Only a lossy WebP writes its size without the minus one.
        for (width, height) in [(0, 2), (3, 0)] {
            assert_eq!(
                inspect(&webp(&[(b"VP8 ", vp8(width, height))])),
                Err(ImageError::Malformed("zero size")),
                "webp {width}x{height}"
            );
        }
    }

    #[test]
    fn inspect_rejects_bytes_that_are_not_an_image() {
        for bytes in [
            &b""[..],
            b"GIF89a\x01\x00\x01\x00",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
            &PNG_SIGNATURE[..7],
        ] {
            assert_eq!(inspect(bytes), Err(ImageError::NotAnImage), "{bytes:?}");
        }
        // A PNG cut inside its header is a broken PNG, not another format.
        let cut = &png(3, 2)[..20];
        assert_eq!(inspect(cut), Err(ImageError::Malformed("truncated")));
        // So is a WebP whose first chunk holds no image or breaks its header.
        let mut no_key_frame = vp8(3, 2);
        no_key_frame[0] |= 1;
        let mut no_start_code = vp8(3, 2);
        no_start_code[3] = 0;
        let mut no_signature = vp8l(3, 2);
        no_signature[0] = 0;
        for (bytes, reason) in [
            (
                webp(&[(b"EXIF", vec![0; 10])]),
                "the first chunk is not VP8, VP8L or VP8X",
            ),
            (webp(&[]), "truncated"),
            (webp(&[(b"VP8 ", vp8(3, 2)[..9].to_vec())]), "truncated"),
            (webp(&[(b"VP8L", vp8l(3, 2)[..4].to_vec())]), "truncated"),
            (webp(&[(b"VP8X", vp8x(0, 3, 2)[..9].to_vec())]), "truncated"),
            (
                webp(&[(b"VP8 ", no_key_frame)]),
                "the VP8 frame is not a key frame",
            ),
            (
                webp(&[(b"VP8 ", no_start_code)]),
                "the VP8 frame is not a key frame",
            ),
            (
                webp(&[(b"VP8L", no_signature)]),
                "the VP8L signature is missing",
            ),
        ] {
            assert_eq!(
                inspect(&bytes),
                Err(ImageError::Malformed(reason)),
                "{reason}"
            );
        }
    }

    fn still(kind: ImageKind, width: u32, height: u32) -> Result<ImageInfo, ImageError> {
        Ok(ImageInfo {
            kind,
            width,
            height,
            animated: false,
        })
    }

    /// A lossy `VP8 ` frame header: the frame tag of a key frame, the start
    /// code and the size in the low 14 bits of each 16, with scale bits set
    /// so a missing mask shows.
    fn vp8(width: u16, height: u16) -> Vec<u8> {
        let mut data = vec![0x50, 0x01, 0x00, 0x9d, 0x01, 0x2a];
        data.extend_from_slice(&(width | 0x4000).to_le_bytes());
        data.extend_from_slice(&(height | 0x8000).to_le_bytes());
        data.extend_from_slice(&[0; 8]);
        data
    }

    #[test]
    fn inspect_reads_webp_lossy_lossless_and_extended_dimensions() {
        let lossy = webp(&[(b"VP8 ", vp8(300, 200))]);
        // In a simple lossy file the size sits in bytes 26 to 29.
        assert_eq!(&lossy[26..30], &[0x2c, 0x41, 0xc8, 0x80]);
        assert_eq!(inspect(&lossy), still(ImageKind::Webp, 300, 200));

        let lossless = webp(&[(b"VP8L", vp8l(1000, 16384))]);
        assert_eq!(inspect(&lossless), still(ImageKind::Webp, 1000, 16384));

        let extended = webp(&[(b"VP8X", vp8x(0x10, 70000, 2)), (b"VP8L", vp8l(3, 2))]);
        assert_eq!(inspect(&extended), still(ImageKind::Webp, 70000, 2));

        // As Chromium writes it: `VP8X`, an `ICCP` profile and a `VP8L` image.
        let chromium = include_bytes!("fixtures/lossless-3x2.webp");
        assert_eq!(inspect(chromium), still(ImageKind::Webp, 3, 2));
    }

    #[test]
    fn inspect_rejects_an_animated_webp_by_flag_and_by_anim_chunk() {
        let animated = Err(ImageError::Malformed("animated"));
        // `VP8X` with its animation flag.
        let flagged = webp(&[(b"VP8X", vp8x(0x02, 3, 2)), (b"VP8L", vp8l(3, 2))]);
        assert_eq!(inspect(&flagged), animated, "flag");
        // Without the flag, but with the chunks of an animation.
        let anim = webp(&[
            (b"VP8X", vp8x(0, 3, 2)),
            (b"ANIM", vec![0; 6]),
            (b"VP8L", vp8l(3, 2)),
        ]);
        assert_eq!(inspect(&anim), animated, "ANIM");
        let anmf = webp(&[
            (b"VP8X", vp8x(0, 3, 2)),
            (b"VP8L", vp8l(3, 2)),
            (b"ANMF", vec![0; 16]),
        ]);
        assert_eq!(inspect(&anmf), animated, "ANMF");
    }

    #[test]
    fn inspect_rejects_a_webp_whose_riff_size_does_not_match() {
        let good = webp_image(3, 2);
        assert_eq!(inspect(&good), still(ImageKind::Webp, 3, 2));
        let declared = u32::from_le_bytes([good[4], good[5], good[6], good[7]]);
        for size in [declared + 2, declared - 2, u32::MAX] {
            let mut bytes = good.clone();
            bytes[4..8].copy_from_slice(&size.to_le_bytes());
            assert_eq!(
                inspect(&bytes),
                Err(ImageError::Malformed(
                    "the RIFF size does not match the file"
                )),
                "{size}"
            );
        }
        // Bytes after the RIFF it declares do not make it longer.
        let mut longer = good.clone();
        longer.extend_from_slice(b"ANIM\0\0\0\0");
        assert_eq!(
            inspect(&longer),
            Err(ImageError::Malformed(
                "the RIFF size does not match the file"
            ))
        );
    }
}
