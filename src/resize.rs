use crate::settings::{ResizeFilter, Settings};
use anyhow::Result;
use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, GenericImageView, RgbaImage};

impl From<ResizeFilter> for ResizeAlg {
    fn from(f: ResizeFilter) -> Self {
        match f {
            ResizeFilter::Nearest => ResizeAlg::Nearest,
            ResizeFilter::Bilinear => ResizeAlg::Convolution(FilterType::Bilinear),
            ResizeFilter::Hamming => ResizeAlg::Convolution(FilterType::Hamming),
            ResizeFilter::CatmullRom => ResizeAlg::Convolution(FilterType::CatmullRom),
            ResizeFilter::Mitchell => ResizeAlg::Convolution(FilterType::Mitchell),
            ResizeFilter::Gaussian => ResizeAlg::Convolution(FilterType::Gaussian),
            ResizeFilter::Lanczos3 => ResizeAlg::Convolution(FilterType::Lanczos3),
        }
    }
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

pub fn resize_image(image: DynamicImage, settings: &Settings) -> Result<RgbaImage> {
    let (width, height) = image.dimensions();

    let (new_width, new_height) = calculate_dimensions(
        width,
        height,
        settings.max_width,
        settings.max_height,
        settings.char_ratio,
    );

    let options = ResizeOptions::new().resize_alg(settings.resize_filter.into());

    let mut resized = DynamicImage::new(new_width, new_height, image.color());
    Resizer::new().resize(&image, &mut resized, &options)?;

    Ok(resized.into_rgba8())
}
