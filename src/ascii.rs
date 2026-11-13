use anyhow::Result;
use crossterm::{
    QueueableCommand,
    style::{self, Color, Print, SetBackgroundColor, SetForegroundColor},
};
use image::{DynamicImage, Rgba, RgbaImage};
use std::io::{self, Write};

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

pub fn load_image(file_path: &str) -> Result<DynamicImage> {
    Ok(image::open(file_path)?)
}

fn get_intensity(image: &RgbaImage, gamma: f32) -> Vec<f32> {
    let has_transparency = image.pixels().any(|p| p[3] < 255);

    if has_transparency {
        return image.pixels().map(|p| f32::from(p[3]) / 255.0).collect();
    }

    let rec709 = |r: u8, g: u8, b: u8| {
        (0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)) / 255.0
    };

    let exponent = 1.0 / gamma;

    image
        .pixels()
        .map(|&Rgba([r, g, b, _])| rec709(r, g, b).powf(exponent))
        .collect()
}

fn gradient_at(intensity: &[f32], width: usize, x: usize, y: usize) -> Gradient {
    let at = |x: usize, y: usize| intensity[y * width + x];

    let (nw, n, ne) = (at(x - 1, y - 1), at(x, y - 1), at(x + 1, y - 1));
    let (w, e) = (at(x - 1, y), at(x + 1, y));
    let (sw, s, se) = (at(x - 1, y + 1), at(x, y + 1), at(x + 1, y + 1));

    Gradient {
        gx: (ne + 2.0 * e + se) - (nw + 2.0 * w + sw),
        gy: (nw + 2.0 * n + ne) - (sw + 2.0 * s + se),
    }
}

fn sobel(intensity: &[f32], width: u32, height: u32) -> Vec<Gradient> {
    let width = width as usize;
    let height = height as usize;

    let mut gradients = vec![Gradient::default(); width * height];

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            gradients[y * width + x] = gradient_at(intensity, width, x, y);
        }
    }

    gradients
}

fn intensity_to_char(intensity: f32, ramp: &[char]) -> char {
    debug_assert!(!ramp.is_empty());

    let i = ((intensity * ramp.len() as f32) as usize).min(ramp.len() - 1);
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
    image: &RgbaImage,
    intensity: &[f32],
    gradients: &[Gradient],
    edge_threshold: f32,
    ramp: &[char],
) -> Vec<AsciiPixel> {
    let mut pixels = Vec::with_capacity(intensity.len());

    for (i, &Rgba([r, g, b, _])) in image.pixels().enumerate() {
        let gradient = gradients[i];

        let ch = if gradient.magnitude() >= edge_threshold {
            edge_char(gradient)
        } else {
            intensity_to_char(intensity[i], ramp)
        };

        pixels.push(AsciiPixel { ch, r, g, b });
    }

    pixels
}

pub fn process_image(
    image: &RgbaImage,
    edge_threshold: f32,
    ramp: &[char],
    gamma: f32,
) -> ProcessedImage {
    let (width, height) = image.dimensions();

    let intensity = get_intensity(image, gamma);

    let gradients = sobel(&intensity, width, height);

    let pixels = to_ascii(image, &intensity, &gradients, edge_threshold, ramp);

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
