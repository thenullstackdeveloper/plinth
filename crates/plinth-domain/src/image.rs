//! Images a template brings in its folder: where they are (`ImagePath`), what
//! they are (`ImageKind`), what they are used for (`ImageUse`) and the bytes
//! the loader read (`ImageAsset`). The domain never reads a file: it only
//! decides which paths and sizes a template may use.

use std::sync::Arc;

use crate::error::DomainError;

/// The image formats a template may use. Both are static raster formats that
/// cannot carry scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageKind {
    Png,
    Webp,
}

impl ImageKind {
    /// The lower-case name, as in the file extension.
    pub fn name(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Webp => "webp",
        }
    }
}

/// The longest path a template may write: four parts of 64 characters and
/// the three `/` between them.
const MAX_PARTS: usize = 4;
const MAX_PART_LEN: usize = 64;

/// Names Windows reserves for devices, whatever the extension.
const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A path to an image, relative to the template's folder: 1 to 4 parts
/// separated by `/`, each starting with a letter or a digit and holding only
/// letters, digits, `.`, `_` and `-`, and ending in `.png` or `.webp`. It
/// cannot climb out of the folder, name a drive or a Windows device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImagePath(String);

impl ImagePath {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let invalid = |reason: &'static str| DomainError::InvalidImagePath {
            path: raw.to_string(),
            reason,
        };
        if raw.contains("..") {
            return Err(invalid("`..` is not allowed"));
        }
        let parts: Vec<&str> = raw.split('/').collect();
        if parts.len() > MAX_PARTS {
            return Err(invalid("it has more than 4 parts"));
        }
        if !parts.iter().all(|part| is_part(part)) {
            return Err(invalid(
                "each part must start with a letter or a digit and hold only letters, digits, `.`, `_` and `-`, up to 64 characters",
            ));
        }
        let file = parts[parts.len() - 1];
        if !(file.ends_with(".png") || file.ends_with(".webp")) {
            return Err(invalid("it must end in `.png` or `.webp`, in lower case"));
        }
        if parts.iter().any(|part| is_reserved(part)) {
            return Err(invalid("it uses a name Windows reserves for a device"));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The format the extension promises.
    pub fn kind(&self) -> ImageKind {
        if self.0.ends_with(".webp") {
            ImageKind::Webp
        } else {
            ImageKind::Png
        }
    }

    /// The parts between the `/`, in order.
    pub fn parts(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }
}

/// `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`.
fn is_part(part: &str) -> bool {
    let mut chars = part.chars();
    let starts_well = chars.next().is_some_and(|c| c.is_ascii_alphanumeric());
    starts_well
        && part.len() <= MAX_PART_LEN
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// A Windows device name, in any case, with or without an extension.
fn is_reserved(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part);
    RESERVED.iter().any(|name| stem.eq_ignore_ascii_case(name))
}

/// What a template uses an image for. Each use has its own limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageUse {
    /// An image that fills the face of a button.
    ButtonImage,
    /// A small image next to a button's label, or alone.
    Icon,
    /// The background of a page.
    Background,
}

/// The largest image a use accepts, by its header and by its size on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageLimits {
    pub max_width: u32,
    pub max_height: u32,
    pub max_bytes: u64,
}

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;

impl ImageUse {
    pub fn limits(self) -> ImageLimits {
        let (side, max_bytes) = match self {
            ImageUse::Icon => (256, 256 * KIB),
            ImageUse::ButtonImage => (1024, MIB),
            ImageUse::Background => (2048, 4 * MIB),
        };
        ImageLimits {
            max_width: side,
            max_height: side,
            max_bytes,
        }
    }
}

impl std::fmt::Display for ImageUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ImageUse::ButtonImage => "a button image",
            ImageUse::Icon => "an icon",
            ImageUse::Background => "a background",
        })
    }
}

/// An image the loader read from the template's folder and found to be a
/// well-formed static image of `kind`, `width` × `height` pixels. Only data:
/// whoever needs the bytes encodes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageAsset {
    pub path: ImagePath,
    pub kind: ImageKind,
    pub width: u32,
    pub height: u32,
    pub bytes: Arc<[u8]>,
}

impl ImageAsset {
    /// Whether the image fits the limits of `use_`, by its header and by its
    /// size.
    pub fn check(&self, use_: ImageUse) -> Result<(), DomainError> {
        let limits = use_.limits();
        let too_large = |measured: String, limit: String| DomainError::ImageTooLarge {
            path: self.path.as_str().to_string(),
            use_,
            measured,
            limit,
        };
        if self.width > limits.max_width || self.height > limits.max_height {
            return Err(too_large(
                format!("{}x{}", self.width, self.height),
                format!("{}x{}", limits.max_width, limits.max_height),
            ));
        }
        let bytes = u64::try_from(self.bytes.len()).unwrap_or(u64::MAX);
        if bytes > limits.max_bytes {
            return Err(too_large(
                format!("{bytes} bytes"),
                format!("{} bytes", limits.max_bytes),
            ));
        }
        Ok(())
    }
}

/// An image of the template, by its index in `Template::images()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageRef(pub usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_path_accepts_a_relative_png_path() {
        for raw in [
            "x.png",
            "img/buttons/fire_ball-2.png",
            "a/b/c/d.webp",
            "1.png",
        ] {
            let path = ImagePath::parse(raw).unwrap_or_else(|e| panic!("`{raw}`: {e}"));
            assert_eq!(path.as_str(), raw);
        }
        assert_eq!(ImagePath::parse("x.png").unwrap().kind(), ImageKind::Png);
        assert_eq!(
            ImagePath::parse("a/x.webp").unwrap().kind(),
            ImageKind::Webp
        );
        let nested = ImagePath::parse("a/b/x.png").unwrap();
        let parts: Vec<&str> = nested.parts().collect();
        assert_eq!(parts, vec!["a", "b", "x.png"]);
    }

    #[test]
    fn image_path_rejects() {
        const PART: &str = "each part must start with a letter or a digit and hold only letters, digits, `.`, `_` and `-`, up to 64 characters";
        const EXTENSION: &str = "it must end in `.png` or `.webp`, in lower case";
        const RESERVED_NAME: &str = "it uses a name Windows reserves for a device";
        let long = format!("{}.png", "a".repeat(61));
        for (raw, reason) in [
            ("../x.png", "`..` is not allowed"),
            ("/x.png", PART),
            ("C:x.png", PART),
            ("a\\x.png", PART),
            ("x.PNG", EXTENSION),
            ("x.gif", EXTENSION),
            ("a/b/c/d/e.png", "it has more than 4 parts"),
            ("nul.png", RESERVED_NAME),
            ("COM1.png", RESERVED_NAME),
            ("", PART),
            (long.as_str(), PART),
        ] {
            assert_eq!(
                ImagePath::parse(raw),
                Err(DomainError::InvalidImagePath {
                    path: raw.to_string(),
                    reason,
                }),
                "`{raw}`"
            );
        }
        assert_eq!(
            ImagePath::parse("../x.png").unwrap_err().to_string(),
            "`../x.png` is not a valid image path: `..` is not allowed"
        );
    }

    fn asset(width: u32, height: u32, bytes: usize) -> ImageAsset {
        ImageAsset {
            path: ImagePath::parse("img/big.png").unwrap(),
            kind: ImageKind::Png,
            width,
            height,
            bytes: vec![0u8; bytes].into(),
        }
    }

    #[test]
    fn a_button_image_over_its_limits_is_rejected() {
        let mib = 1024 * 1024;
        assert_eq!(asset(1024, 1024, mib).check(ImageUse::ButtonImage), Ok(()));
        assert_eq!(
            asset(1025, 1, 10)
                .check(ImageUse::ButtonImage)
                .unwrap_err()
                .to_string(),
            "`img/big.png` is too large for a button image: 1025x1 > 1024x1024"
        );
        assert_eq!(
            asset(1, 1025, 10)
                .check(ImageUse::ButtonImage)
                .unwrap_err()
                .to_string(),
            "`img/big.png` is too large for a button image: 1x1025 > 1024x1024"
        );
        assert_eq!(
            asset(1, 1, mib + 1)
                .check(ImageUse::ButtonImage)
                .unwrap_err()
                .to_string(),
            "`img/big.png` is too large for a button image: 1048577 bytes > 1048576 bytes"
        );
    }
}
