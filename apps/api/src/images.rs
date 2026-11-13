//! Upload validation and processing.
//!
//! Uploads are bounded twice: by byte size before decoding, and by pixel budget
//! during it, so a small file that decompresses to an enormous bitmap cannot
//! exhaust memory. Output is WebP at a long edge of 2560px, which keeps both
//! the storage bill and the browser's decode cost of a full-viewport image
//! reasonable.

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, ImageReader};

use crate::error::{AppError, AppResult};

pub const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 2560;
const WEBP_QUALITY: f32 = 82.0;

/// Ceiling on decoded pixel data, independent of the compressed size.
const MAX_DECODED_BYTES: u64 = 512 * 1024 * 1024;

pub struct ProcessedImage {
    pub bytes: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

/// Omits the pixel payload: a failing assertion should print dimensions, not
/// a megabyte of WebP.
impl std::fmt::Debug for ProcessedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessedImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &format_args!("{} bytes", self.bytes.len()))
            .finish()
    }
}

impl ProcessedImage {
    pub const CONTENT_TYPE: &'static str = "image/webp";
    pub const EXTENSION: &'static str = "webp";
}

/// Validates and normalises an uploaded image.
pub fn process(raw: &[u8]) -> AppResult<ProcessedImage> {
    if raw.is_empty() {
        return Err(AppError::bad_request("the uploaded file is empty"));
    }
    if raw.len() > MAX_UPLOAD_BYTES {
        return Err(AppError::Payload(format!(
            "images must be {} MB or smaller",
            MAX_UPLOAD_BYTES / (1024 * 1024)
        )));
    }

    let format = image::guess_format(raw)
        .map_err(|_| AppError::bad_request("could not read that file as an image"))?;

    if !matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP
    ) {
        return Err(AppError::bad_request(
            "images must be JPEG, PNG, or WebP",
        ));
    }

    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODED_BYTES);

    let mut reader = ImageReader::new(Cursor::new(raw));
    reader.set_format(format);
    reader.limits(limits);

    let image = reader
        .decode()
        .map_err(|e| AppError::bad_request(format!("could not decode that image: {e}")))?;

    let image = downscale(image);
    let (width, height) = (image.width(), image.height());

    if width == 0 || height == 0 {
        return Err(AppError::bad_request("the image has no dimensions"));
    }

    let bytes = encode_webp(&image)?;

    Ok(ProcessedImage {
        bytes,
        width: width as i32,
        height: height as i32,
    })
}

/// Shrinks so the longer edge is at most [`MAX_DIMENSION`], preserving aspect
/// ratio. Smaller images pass through untouched — upscaling would add bytes
/// without adding detail.
fn downscale(image: DynamicImage) -> DynamicImage {
    let (w, h) = (image.width(), image.height());
    if w <= MAX_DIMENSION && h <= MAX_DIMENSION {
        return image;
    }
    // `resize` fits within the bounding box while preserving the ratio.
    image.resize(MAX_DIMENSION, MAX_DIMENSION, image::imageops::FilterType::Lanczos3)
}

fn encode_webp(image: &DynamicImage) -> AppResult<Vec<u8>> {
    let (w, h) = (image.width(), image.height());

    // Encoding from RGB where there is no alpha avoids carrying a pointless
    // opaque alpha channel through the encoder.
    let encoded = if image.color().has_alpha() {
        let buf = image.to_rgba8();
        webp::Encoder::from_rgba(buf.as_raw(), w, h).encode(WEBP_QUALITY)
    } else {
        let buf = image.to_rgb8();
        webp::Encoder::from_rgb(buf.as_raw(), w, h).encode(WEBP_QUALITY)
    };

    Ok(encoded.to_vec())
}

/// Storage key for a processed picture.
pub fn storage_key(id: uuid::Uuid) -> String {
    format!("pictures/{id}.{}", ProcessedImage::EXTENSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn png_of(w: u32, h: u32) -> Vec<u8> {
        let mut img = RgbImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn png_is_accepted_and_re_encoded_to_webp() {
        let processed = process(&png_of(100, 80)).unwrap();
        assert_eq!((processed.width, processed.height), (100, 80));
        // WebP files start with RIFF....WEBP.
        assert_eq!(&processed.bytes[0..4], b"RIFF");
        assert_eq!(&processed.bytes[8..12], b"WEBP");
    }

    #[test]
    fn oversized_images_are_downscaled_preserving_aspect_ratio() {
        let processed = process(&png_of(5000, 2500)).unwrap();
        assert_eq!(processed.width, MAX_DIMENSION as i32);
        assert_eq!(processed.height, (MAX_DIMENSION / 2) as i32);
    }

    #[test]
    fn small_images_are_not_upscaled() {
        let processed = process(&png_of(64, 48)).unwrap();
        assert_eq!((processed.width, processed.height), (64, 48));
    }

    #[test]
    fn empty_and_non_image_payloads_are_rejected() {
        assert!(process(&[]).is_err());
        assert!(process(b"this is not an image at all").is_err());
    }

    #[test]
    fn oversized_payloads_are_rejected_before_decoding() {
        let huge = vec![0u8; MAX_UPLOAD_BYTES + 1];
        let err = process(&huge).unwrap_err();
        assert!(matches!(err, AppError::Payload(_)));
    }

    #[test]
    fn unsupported_formats_are_rejected() {
        // A valid GIF header, which decodes but is not an accepted format.
        let gif = b"GIF89a\x01\x00\x01\x00\x00\xff\x00,\x00\x00\x00\x00\
                    \x01\x00\x01\x00\x00\x02\x02D\x01\x00;";
        let err = process(gif).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn storage_keys_are_namespaced_and_safe() {
        let key = storage_key(uuid::Uuid::nil());
        assert!(key.starts_with("pictures/"));
        assert!(key.ends_with(".webp"));
        assert!(!key.contains(".."));
    }
}
