use clap::{Parser, ValueEnum};

/// The maximum allowed size of the ASCII output
const MAX_DIMENSION: u32 = 1024;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

/// The arguments a user can provide to the program
#[derive(Debug, Parser)]
#[command(
    version,
    about = "Convert images to ASCII art",
    long_about = "This program takes an image and converts it into ASCII art"
)]
struct Args {
    /// Path to the image file
    file_path: String,

    /// Maximum width of the ASCII output
    #[arg(short = 'W', long = "max-width", value_parser = clap::value_parser!(u32).range(1..=MAX_DIMENSION as i64))]
    max_width: Option<u32>,

    /// Maximum height of the ASCII output
    #[arg(short = 'H', long = "max-height", value_parser = clap::value_parser!(u32).range(1..=MAX_DIMENSION as i64))]
    max_height: Option<u32>,

    /// Character aspect ratio
    #[arg(short = 'c', long = "char-ratio", default_value_t = 2.0)]
    char_ratio: f32,

    /// The filter to be used when resizing the image
    #[arg(long = "resize-filter", value_enum, default_value_t = ResizeFilter::Lanczos3)]
    resize_filter: ResizeFilter,
}

/// All the processed settings that came from the program arguments
#[derive(Debug)]
pub struct Settings {
    /// Path to the image file
    pub file_path: String,

    /// Maximum width of the ASCII output
    pub max_width: u32,

    /// Maximum height of the ASCII output
    pub max_height: u32,

    /// Character aspect ratio
    pub char_ratio: f32,

    /// The filter to be used when resizing the image
    pub resize_filter: ResizeFilter,
}

impl From<Args> for Settings {
    fn from(args: Args) -> Self {
        const DEFAULT_WIDTH: u32 = 80;
        const DEFAULT_HEIGHT: u32 = 60;

        Settings {
            file_path: args.file_path,
            max_width: args.max_width.unwrap_or(DEFAULT_WIDTH),
            max_height: args.max_height.unwrap_or(DEFAULT_HEIGHT),
            char_ratio: args.char_ratio,
            resize_filter: args.resize_filter,
        }
    }
}

/// Parse the CLI arguments and resolve them into Settings
pub fn parse_args() -> Settings {
    Args::parse().into()
}
