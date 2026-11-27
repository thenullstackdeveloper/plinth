//! What an image file really is, read from its first bytes: the format, the
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

/// Reads the format and the size of an image from its header. A PNG must
/// start with `IHDR`, declare a size other than zero and carry no `acTL`
/// chunk (an animated PNG).
pub fn inspect(bytes: &[u8]) -> Result<ImageInfo, ImageError> {
    match bytes.strip_prefix(PNG_SIGNATURE) {
        Some(chunks) => inspect_png(chunks),
        None => Err(ImageError::NotAnImage),
    }
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
}

#[cfg(test)]
mod tests {
    use super::test_images::{chunk, idat, ihdr, png, png_from};
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
    }
}
