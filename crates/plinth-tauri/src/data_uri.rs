//! A template's images as `data:` URIs, the only image source the window's
//! UI accepts. The base64 is written here (RFC 4648, standard alphabet with
//! padding) rather than pulled in as a dependency.

use plinth_domain::{ImageAsset, ImageKind};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` in base64 with `=` padding.
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let b = [
            group[0],
            group.get(1).copied().unwrap_or(0),
            group.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let sextet = |shift: u32| char::from(ALPHABET[((n >> shift) & 0x3F) as usize]);
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if group.len() > 1 { sextet(6) } else { '=' });
        out.push(if group.len() > 2 { sextet(0) } else { '=' });
    }
    out
}

/// The image as `data:image/<png|webp>;base64,…`.
pub fn of(asset: &ImageAsset) -> String {
    let mime = match asset.kind {
        ImageKind::Png => "image/png",
        ImageKind::Webp => "image/webp",
    };
    format!("data:{mime};base64,{}", base64(&asset.bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        // RFC 4648, section 10.
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected, "`{input}`");
        }
        // Every sextet value, so no letter of the alphabet is wrong.
        let all: Vec<u8> = (0u8..=255).collect();
        let encoded = base64(&all);
        assert!(encoded.starts_with("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8g"));
        assert!(encoded.ends_with("8/T19vf4+fr7/P3+/w=="));
    }
}
