use clap::{Parser, ValueEnum};
use crossterm::terminal::size;
use std::io::{self, IsTerminal};

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

/// When to colour the output
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColourMode {
    /// Colour only when printing to a terminal
    Auto,

    /// Always colour, even when piped or redirected
    Always,

    /// Never colour
    Never,
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

    /// Try to scale the image so as to fit the terminal's dimensions
    #[arg(long = "fit-terminal")]
    fit_terminal: bool,

    /// The filter to be used when resizing the image
    #[arg(long = "resize-filter", value_enum, default_value_t = ResizeFilter::Lanczos3)]
    resize_filter: ResizeFilter,

    /// When to use colour in the output
    #[arg(long = "colour", value_enum, default_value_t = ColourMode::Auto)]
    colour: ColourMode,
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

    /// Whether the output should be coloured
    pub colour: bool,
}

impl From<Args> for Settings {
    fn from(args: Args) -> Self {
        const DEFAULT_WIDTH: u32 = 80;
        const DEFAULT_HEIGHT: u32 = 60;

        let mut max_width = args.max_width;
        let mut max_height = args.max_height;

        // Giving precedence to width and height values provided manually
        if args.fit_terminal
            && let Ok((w, h)) = size()
        {
            if max_width.is_none() && w > 0 {
                max_width = Some(w as u32);
            }
            if max_height.is_none() && h > 0 {
                max_height = Some(h as u32);
            }
        }

        let max_width = max_width.unwrap_or(DEFAULT_WIDTH).min(MAX_DIMENSION);
        let max_height = max_height.unwrap_or(DEFAULT_HEIGHT).min(MAX_DIMENSION);

        let colour = match args.colour {
            ColourMode::Auto => io::stdout().is_terminal(),
            ColourMode::Always => true,
            ColourMode::Never => false,
        };

        Settings {
            file_path: args.file_path,
            max_width,
            max_height,
            char_ratio: args.char_ratio,
            resize_filter: args.resize_filter,
            colour,
        }
    }
}

/// Parse the CLI arguments and resolve them into Settings
pub fn parse_args() -> Settings {
    Args::parse().into()
}
