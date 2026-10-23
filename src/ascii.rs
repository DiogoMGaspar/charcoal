use crate::settings::{ResizeFilter, Settings};
use anyhow::Result;
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use std::io::{self, Write};

const ASCII_CHARS: &[u8] = b" .-=+*x#$&X@";

impl From<ResizeFilter> for FilterType {
    fn from(f: ResizeFilter) -> Self {
        match f {
            ResizeFilter::Nearest => FilterType::Nearest,
            ResizeFilter::Triangle => FilterType::Triangle,
            ResizeFilter::CatmullRom => FilterType::CatmullRom,
            ResizeFilter::Gaussian => FilterType::Gaussian,
            ResizeFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

/// ASCII representation of a pixel
struct AsciiPixel {
    /// Character
    ch: u8,
}

/// Fully processed image
pub struct ProcessedImage {
    /// Width of the image
    width: u32,

    /// Height of the image
    height: u32,

    /// Pixel data
    pixels: Vec<AsciiPixel>,
}

/// Attempts to load an image from a file
pub fn load_image(file_path: &str) -> Result<DynamicImage> {
    Ok(image::open(file_path)?)
}

/// Calculates the dimensions the image should be resized to
fn calculate_dimensions(
    image_width: u32,
    image_height: u32,
    max_width: u32,
    max_height: u32,
    char_ratio: f32,
) -> (u32, u32) {
    let iw = image_width as f32;
    let ih = image_height as f32;
    let mw = max_width as f32;
    let mh = max_height as f32;

    let new_height = ih * mw / (char_ratio * iw);

    if new_height <= mh {
        (max_width, (new_height.round() as u32).max(1))
    } else {
        let width = char_ratio * iw * mh / ih;
        ((width.round() as u32).max(1), max_height)
    }
}

/// Resizes an image to the correct number of characters
pub fn resize_image(image: &DynamicImage, settings: &Settings) -> DynamicImage {
    let (width, height) = image.dimensions();

    let (new_width, new_height) = calculate_dimensions(
        width,
        height,
        settings.max_width,
        settings.max_height,
        settings.char_ratio,
    );

    image.resize_exact(new_width, new_height, settings.resize_filter.into())
}

/// Gets the luminance of an image
fn get_luminance(image: &image::RgbImage) -> Vec<f32> {
    let mut luminance = Vec::with_capacity((image.width() * image.height()) as usize);

    // Using Rec. 709 luminance
    for pixel in image.pixels() {
        let [r, g, b] = pixel.0;
        let luma = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
        luminance.push(luma.clamp(0.0, 1.0));
    }

    luminance
}

/// Convert the given luminance values into the adequate ASCII characters
fn to_ascii(luminance: &[f32]) -> Vec<AsciiPixel> {
    luminance
        .iter()
        .map(|&l| AsciiPixel {
            ch: luminance_to_char(l),
        })
        .collect()
}

/// Processes an image
pub fn process_image(image: &DynamicImage) -> ProcessedImage {
    let rgb = image.to_rgb8();

    let (width, height) = (rgb.width(), rgb.height());

    let luminance = get_luminance(&rgb);

    let pixels = to_ascii(&luminance);

    ProcessedImage {
        width,
        height,
        pixels,
    }
}

/// Convert a luminance value to its respective character
#[inline]
fn luminance_to_char(luminance: f32) -> u8 {
    let i = (luminance * (ASCII_CHARS.len() - 1) as f32) as usize;
    ASCII_CHARS[i]
}

/// Prints an image as ASCII characters
pub fn print_image(image: &ProcessedImage) -> io::Result<()> {
    let mut stdout = io::BufWriter::new(io::stdout());

    let width = image.width as usize;
    let height = image.height as usize;

    for y in 0..height {
        let row = y * width;

        for x in 0..width {
            let i = row + x;
            let pixel = &image.pixels[i];

            stdout.write_all(&[pixel.ch])?;
        }
        stdout.write_all(b"\n")?;
    }

    stdout.flush()?;
    Ok(())
}
