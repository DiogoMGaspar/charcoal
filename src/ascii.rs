use crate::settings::{ResizeFilter, Settings};
use anyhow::Result;
use crossterm::{
    QueueableCommand,
    style::{self, Color, Print, SetBackgroundColor, SetForegroundColor},
};
use image::{
    DynamicImage, RgbImage,
    imageops::{self, FilterType},
};
use std::io::{self, Write};

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
    ch: char,

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

/// Blends transparent pixels over black and drops the alpha channel
fn flatten_onto_black(image: DynamicImage) -> RgbImage {
    if !image.color().has_alpha() {
        return image.into_rgb8();
    }

    let rgba = image.into_rgba8();
    let mut rgb = RgbImage::new(rgba.width(), rgba.height());

    for (dst, src) in rgb.pixels_mut().zip(rgba.pixels()) {
        let [r, g, b, a] = src.0;
        let a = a as u16;

        let blend = |c: u8| ((c as u16 * a + 127) / 255) as u8;

        *dst = image::Rgb([blend(r), blend(g), blend(b)]);
    }

    rgb
}

/// Attempts to load an image from a file as RGB
pub fn load_image(file_path: &str) -> Result<RgbImage> {
    Ok(flatten_onto_black(image::open(file_path)?))
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
pub fn resize_image(image: &RgbImage, settings: &Settings) -> RgbImage {
    let (width, height) = image.dimensions();

    let (new_width, new_height) = calculate_dimensions(
        width,
        height,
        settings.max_width,
        settings.max_height,
        settings.char_ratio,
    );

    imageops::resize(image, new_width, new_height, settings.resize_filter.into())
}

/// Gets the luminance of an image
fn get_luminance(image: &RgbImage) -> Vec<f32> {
    let mut luminance = Vec::with_capacity((image.width() * image.height()) as usize);

    // Using Rec. 709 luminance
    for pixel in image.pixels() {
        let [r, g, b] = pixel.0;
        let luma = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
        luminance.push(luma.clamp(0.0, 1.0));
    }

    luminance
}

/// Applies the Sobel operator over an image to get the magnitude and angle of its edges
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
fn luminance_to_char(luminance: f32, ramp: &[char]) -> char {
    let i = ((luminance * ramp.len() as f32) as usize).min(ramp.len() - 1);
    ramp[i]
}

/// Gets the appropriate edge character based off the angle
fn edge_char(angle: f32) -> char {
    let a = (angle + std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI);

    match ((a / std::f32::consts::FRAC_PI_4).round() as i32) & 3 {
        0 => '|',
        1 => '\\',
        2 => '_',
        _ => '/',
    }
}

/// Convert the given luminance values into the adequate ASCII characters and colours
fn to_ascii(
    image: &image::RgbImage,
    luminance: &[f32],
    edges: &[f32],
    angles: &[f32],
    edge_threshold: f32,
    ramp: &[char],
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
            luminance_to_char(luminance[i], ramp)
        };

        pixels.push(AsciiPixel { ch, r, g, b });
    }

    pixels
}

/// Processes an image
pub fn process_image(image: &RgbImage, edge_threshold: f32, ramp: &[char]) -> ProcessedImage {
    let (width, height) = image.dimensions();

    let luminance = get_luminance(image);

    let (edges, angles) = sobel(&luminance, width, height, edge_threshold);

    let pixels = to_ascii(image, &luminance, &edges, &angles, edge_threshold, ramp);

    ProcessedImage {
        width,
        height,
        pixels,
    }
}

/// Scales a colour channel to the given percentage of its value
fn scale(channel: u8, fraction: f32) -> u8 {
    (channel as f32 * fraction).round() as u8
}

/// Prints an image as ASCII characters
pub fn print_image(image: &ProcessedImage, colour: bool, background: f32) -> io::Result<()> {
    let mut stdout = io::BufWriter::new(io::stdout());

    let width = image.width as usize;
    let height = image.height as usize;

    let use_background = colour && background > 0.0;

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

            if use_background {
                stdout.queue(SetBackgroundColor(Color::Rgb {
                    r: scale(pixel.r, background),
                    g: scale(pixel.g, background),
                    b: scale(pixel.b, background),
                }))?;
            }

            stdout.queue(Print(pixel.ch))?;
        }

        if use_background {
            stdout.queue(style::ResetColor)?;
        }

        stdout.queue(Print('\n'))?;
    }

    if colour {
        stdout.queue(style::ResetColor)?;
    }

    stdout.flush()?;
    Ok(())
}
