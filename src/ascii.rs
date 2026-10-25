use crate::settings::{ResizeFilter, Settings};
use anyhow::Result;
use crossterm::{
    QueueableCommand,
    style::{self, Color, Print, SetBackgroundColor, SetForegroundColor},
};
use image::{
    DynamicImage, Rgb, RgbImage,
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

#[derive(Clone, Copy, Default)]
struct Gradient {
    gx: f32,
    gy: f32,
}

impl Gradient {
    fn magnitude(self) -> f32 {
        const MAX_SOBEL: f32 = 4.0 * std::f32::consts::SQRT_2;

        (self.gx.hypot(self.gy) / MAX_SOBEL).clamp(0.0, 1.0)
    }
}

struct AsciiPixel {
    ch: char,
    r: u8,
    g: u8,
    b: u8,
}

impl AsciiPixel {
    fn to_colour(&self) -> Color {
        Color::Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }

    fn to_scaled_colour(&self, fraction: f32) -> Color {
        let scale = |c: u8| (c as f32 * fraction).round() as u8;

        Color::Rgb {
            r: scale(self.r),
            g: scale(self.g),
            b: scale(self.b),
        }
    }
}

pub struct ProcessedImage {
    width: u32,
    pixels: Vec<AsciiPixel>,
}

fn flatten_onto_black(image: DynamicImage) -> RgbImage {
    if !image.color().has_alpha() {
        return image.into_rgb8();
    }

    let blend = |c: u8, a: u8| ((u16::from(c) * u16::from(a) + 127) / 255) as u8;

    let rgba = image.into_rgba8();

    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        image::Rgb([r, g, b].map(|c| blend(c, a)))
    })
}

pub fn load_image(file_path: &str) -> Result<RgbImage> {
    Ok(flatten_onto_black(image::open(file_path)?))
}

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

pub fn resize_image(image: RgbImage, settings: &Settings) -> RgbImage {
    let (width, height) = image.dimensions();

    let (new_width, new_height) = calculate_dimensions(
        width,
        height,
        settings.max_width,
        settings.max_height,
        settings.char_ratio,
    );

    imageops::resize(&image, new_width, new_height, settings.resize_filter.into())
}

fn get_luminance(image: &RgbImage) -> Vec<f32> {
    let rec709 = |r: u8, g: u8, b: u8| {
        (0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)) / 255.0
    };

    image
        .pixels()
        .map(|&Rgb([r, g, b])| rec709(r, g, b))
        .collect()
}

fn gradient_at(luminance: &[f32], width: usize, x: usize, y: usize) -> Gradient {
    let at = |x: usize, y: usize| luminance[y * width + x];

    let (nw, n, ne) = (at(x - 1, y - 1), at(x, y - 1), at(x + 1, y - 1));
    let (w, e) = (at(x - 1, y), at(x + 1, y));
    let (sw, s, se) = (at(x - 1, y + 1), at(x, y + 1), at(x + 1, y + 1));

    Gradient {
        gx: (ne + 2.0 * e + se) - (nw + 2.0 * w + sw),
        gy: (nw + 2.0 * n + ne) - (sw + 2.0 * s + se),
    }
}

fn sobel(luminance: &[f32], width: u32, height: u32) -> Vec<Gradient> {
    let width = width as usize;
    let height = height as usize;

    let mut gradients = vec![Gradient::default(); width * height];

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            gradients[y * width + x] = gradient_at(luminance, width, x, y);
        }
    }

    gradients
}

fn luminance_to_char(luminance: f32, ramp: &[char]) -> char {
    debug_assert!(!ramp.is_empty());

    let i = ((luminance * ramp.len() as f32) as usize).min(ramp.len() - 1);
    ramp[i]
}

fn edge_char(Gradient { gx, gy }: Gradient) -> char {
    const TAN_FRAC_PI_8: f32 = std::f32::consts::SQRT_2 - 1.0;

    let (ax, ay) = (gx.abs(), gy.abs());

    if ay <= ax * TAN_FRAC_PI_8 {
        '|'
    } else if ax <= ay * TAN_FRAC_PI_8 {
        '_'
    } else if gx * gy > 0.0 {
        '\\'
    } else {
        '/'
    }
}

fn to_ascii(
    image: &RgbImage,
    luminance: &[f32],
    gradients: &[Gradient],
    edge_threshold: f32,
    ramp: &[char],
) -> Vec<AsciiPixel> {
    let mut pixels = Vec::with_capacity(luminance.len());

    for (i, &Rgb([r, g, b])) in image.pixels().enumerate() {
        let gradient = gradients[i];

        let ch = if gradient.magnitude() >= edge_threshold {
            edge_char(gradient)
        } else {
            luminance_to_char(luminance[i], ramp)
        };

        pixels.push(AsciiPixel { ch, r, g, b });
    }

    pixels
}

pub fn process_image(image: &RgbImage, edge_threshold: f32, ramp: &[char]) -> ProcessedImage {
    let (width, height) = image.dimensions();

    let luminance = get_luminance(image);

    let gradients = sobel(&luminance, width, height);

    let pixels = to_ascii(image, &luminance, &gradients, edge_threshold, ramp);

    ProcessedImage { width, pixels }
}

pub fn print_image(image: &ProcessedImage, colour: bool, background: f32) -> io::Result<()> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    let use_background = colour && background > 0.0;

    for row in image.pixels.chunks(image.width as usize) {
        for pixel in row {
            if colour {
                out.queue(SetForegroundColor(pixel.to_colour()))?;
            }
            if use_background {
                out.queue(SetBackgroundColor(pixel.to_scaled_colour(background)))?;
            }
            out.queue(Print(pixel.ch))?;
        }

        if use_background {
            out.queue(style::ResetColor)?;
        }
        out.queue(Print('\n'))?;
    }

    if colour {
        out.queue(style::ResetColor)?;
    }

    out.flush()
}
