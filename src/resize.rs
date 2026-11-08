use crate::settings::{ResizeFilter, Settings};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer, images::Image};
use image::{DynamicImage, RgbImage, RgbaImage};

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

fn resize_raw(
    raw: Vec<u8>,
    pixel_type: PixelType,
    (width, height): (u32, u32),
    (new_width, new_height): (u32, u32),
    options: &ResizeOptions,
) -> Vec<u8> {
    let src = Image::from_vec_u8(width, height, raw, pixel_type).unwrap();
    let mut dst = Image::new(new_width, new_height, pixel_type);
    Resizer::new().resize(&src, &mut dst, options).unwrap();

    dst.into_vec()
}

pub fn resize_image(image: DynamicImage, settings: &Settings) -> RgbaImage {
    let (width, height) = (image.width(), image.height());

    let (new_width, new_height) = calculate_dimensions(
        width,
        height,
        settings.max_width,
        settings.max_height,
        settings.char_ratio,
    );

    let options = ResizeOptions::new().resize_alg(settings.resize_filter.into());

    let (from, to) = ((width, height), (new_width, new_height));

    if image.color().has_alpha() {
        let raw = image.into_rgba8().into_raw();
        let resized = resize_raw(raw, PixelType::U8x4, from, to, &options);

        RgbaImage::from_raw(new_width, new_height, resized).unwrap()
    } else {
        let raw = image.into_rgb8().into_raw();
        let resized = resize_raw(raw, PixelType::U8x3, from, to, &options);

        let small = RgbImage::from_raw(new_width, new_height, resized).unwrap();
        DynamicImage::from(small).into_rgba8()
    }
}
