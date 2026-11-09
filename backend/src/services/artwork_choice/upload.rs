//! Checking an uploaded picture before it is stored.
//!
//! JPEG, PNG and WebP up to 20 MiB, at least 100 pixels on the short side, and no more than 100
//! megapixels decoded (a small file can still decode to a huge image). The picture is decoded and
//! encoded again, which drops its metadata (camera, location) and anything that only claims to be
//! an image. A picture wider than 3840 pixels is scaled down to that, the most any screen asks of
//! the cache; the cache's smaller sizes are made from it when it is stored.

use image::{DynamicImage, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use thiserror::Error;

/// The largest file accepted.
pub const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;
/// The smallest short side accepted.
pub const MIN_UPLOAD_SIDE: u32 = 100;
/// The widest picture kept.
pub const MAX_STORED_WIDTH: u32 = 3840;
const MAX_DECODED_PIXELS: u64 = 100_000_000;
const UPLOAD_JPEG_QUALITY: u8 = 90;

/// Why a file can't be used. The message is shown to the person who uploaded it.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UploadError {
    #[error("The file is empty")]
    Empty,
    #[error("The file is larger than 20 MB")]
    TooLarge,
    #[error("Only JPEG, PNG and WebP pictures can be used")]
    UnsupportedType,
    #[error("The picture couldn't be read")]
    Unreadable,
    #[error("The picture is too small: it needs to be at least 100 pixels on each side")]
    TooSmall,
    #[error("The picture has too many pixels")]
    TooManyPixels,
}

/// A checked picture, ready to store.
#[derive(Debug, Clone)]
pub struct PreparedUpload {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub width: i32,
    pub height: i32,
    /// Of the stored bytes, so the same picture uploaded twice is recognised.
    pub sha256_hex: String,
}

/// Check and normalise an uploaded picture.
pub fn prepare_upload(bytes: &[u8]) -> Result<PreparedUpload, UploadError> {
    if bytes.is_empty() {
        return Err(UploadError::Empty);
    }
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(UploadError::TooLarge);
    }
    let format = image::guess_format(bytes).map_err(|_| UploadError::UnsupportedType)?;
    if !matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP
    ) {
        return Err(UploadError::UnsupportedType);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| UploadError::Unreadable)?;
    if u64::from(width) * u64::from(height) > MAX_DECODED_PIXELS {
        return Err(UploadError::TooManyPixels);
    }
    if width.min(height) < MIN_UPLOAD_SIDE {
        return Err(UploadError::TooSmall);
    }
    reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODED_PIXELS * 4 + 64 * 1024 * 1024);
    reader.limits(limits);
    let mut image = reader.decode().map_err(|_| UploadError::Unreadable)?;
    if image.width() > MAX_STORED_WIDTH {
        let height = ((u64::from(image.height()) * u64::from(MAX_STORED_WIDTH))
            / u64::from(image.width()))
        .max(1) as u32;
        image = image.resize_exact(
            MAX_STORED_WIDTH,
            height,
            image::imageops::FilterType::Lanczos3,
        );
    }
    let (data, mime_type) = encode(&image)?;
    let sha256_hex = hex::encode(Sha256::digest(&data));
    Ok(PreparedUpload {
        width: i32::try_from(image.width()).map_err(|_| UploadError::TooManyPixels)?,
        height: i32::try_from(image.height()).map_err(|_| UploadError::TooManyPixels)?,
        data,
        mime_type: mime_type.to_string(),
        sha256_hex,
    })
}

/// A picture with transparency (a logo) stays PNG; everything else becomes JPEG.
fn encode(image: &DynamicImage) -> Result<(Vec<u8>, &'static str), UploadError> {
    let mut out = Cursor::new(Vec::new());
    if image.color().has_alpha() {
        image
            .write_to(&mut out, ImageFormat::Png)
            .map_err(|_| UploadError::Unreadable)?;
        Ok((out.into_inner(), "image/png"))
    } else {
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, UPLOAD_JPEG_QUALITY)
            .encode_image(&image.to_rgb8())
            .map_err(|_| UploadError::Unreadable)?;
        Ok((out.into_inner(), "image/jpeg"))
    }
}
