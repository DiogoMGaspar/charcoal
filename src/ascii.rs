use crate::settings::{ResizeFilter, Settings};
use anyhow::Result;
use crossterm::{
    QueueableCommand,
    style::{self, Color, Print, SetForegroundColor},
};
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

    /// Red value
    r: u8,

    /// Green value
    g: u8,

    /// Blue value
    b: u8,
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

/// Applies the Sobel operator over an image
///
/// Gets the magnitude and angle of the edges of the image
fn sobel(luminance: &[f32], width: u32, height: u32, edge_threshold: f32) -> (Vec<f32>, Vec<f32>) {
    let width = width as usize;
    let height = height as usize;

    let mut magnitude = vec![0.0; width * height];
    let mut angle = vec![0.0; width * height];

    let max_sobel = (32.0_f32).sqrt();

    for y in 1..height - 1 {
        let prev_row = (y - 1) * width;
        let curr_row = y * width;
        let next_row = (y + 1) * width;

        for x in 1..width - 1 {
            let x_left = x - 1;
            let x_right = x + 1;

            let gx = (luminance[prev_row + x_right] - luminance[prev_row + x_left])
                + 2.0 * (luminance[curr_row + x_right] - luminance[curr_row + x_left])
                + (luminance[next_row + x_right] - luminance[next_row + x_left]);

            let gy = (luminance[prev_row + x_left]
                + 2.0 * luminance[prev_row + x]
                + luminance[prev_row + x_right])
                - (luminance[next_row + x_left]
                    + 2.0 * luminance[next_row + x]
                    + luminance[next_row + x_right]);

            let i = curr_row + x;

            let raw_mag = gx.hypot(gy);
            let mag = (raw_mag / max_sobel).clamp(0.0, 1.0);
            magnitude[i] = mag;

            if mag >= edge_threshold {
                angle[i] = gy.atan2(gx);
            }
        }
    }

    (magnitude, angle)
}

/// Convert a luminance value to its respective character
#[inline]
fn luminance_to_char(luminance: f32) -> u8 {
    let i = (luminance * (ASCII_CHARS.len() - 1) as f32) as usize;
    ASCII_CHARS[i]
}

/// Gets the appropriate edge character based off the angle
fn edge_char(angle: f32) -> u8 {
    let a = (angle + std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI);

    match ((a / std::f32::consts::FRAC_PI_4).round() as i32) & 3 {
        0 => b'|',
        1 => b'\\',
        2 => b'_',
        _ => b'/',
    }
}

/// Convert the given luminance values into the adequate ASCII characters and colours
fn to_ascii(
    image: &image::RgbImage,
    luminance: &[f32],
    edges: &[f32],
    angles: &[f32],
    edge_threshold: f32,
) -> Vec<AsciiPixel> {
    let (width, height) = (image.width() as usize, image.height() as usize);
    let length = width * height;
    let mut pixels = Vec::with_capacity(length);

    let raw = image.as_raw();

    for i in 0..length {
        let base = i * 3;
        let r = raw[base];
        let g = raw[base + 1];
        let b = raw[base + 2];

        let ch = if edges[i] >= edge_threshold {
            edge_char(angles[i])
        } else {
            luminance_to_char(luminance[i])
        };

        pixels.push(AsciiPixel { ch, r, g, b });
    }

    pixels
}

/// Processes an image
pub fn process_image(image: &DynamicImage, edge_threshold: f32) -> ProcessedImage {
    let rgb_image = image.to_rgb8();

    let (width, height) = (rgb_image.width(), rgb_image.height());

    let luminance = get_luminance(&rgb_image);

    let (edges, angles) = sobel(&luminance, width, height, edge_threshold);

    let pixels = to_ascii(&rgb_image, &luminance, &edges, &angles, edge_threshold);

    ProcessedImage {
        width,
        height,
        pixels,
    }
}

/// Prints an image as ASCII characters
pub fn print_image(image: &ProcessedImage, colour: bool) -> io::Result<()> {
    let mut stdout = io::BufWriter::new(io::stdout());

    let width = image.width as usize;
    let height = image.height as usize;

    for y in 0..height {
        let row = y * width;

        for x in 0..width {
            let i = row + x;
            let pixel = &image.pixels[i];

            if colour {
                stdout.queue(SetForegroundColor(Color::Rgb {
                    r: pixel.r,
                    g: pixel.g,
                    b: pixel.b,
                }))?;
            }

            stdout.queue(Print(pixel.ch as char))?;
        }
        stdout.queue(Print('\n'))?;
    }

    if colour {
        stdout.queue(style::ResetColor)?;
    }

    stdout.flush()?;
    Ok(())
}
