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

/// Built-in character ramps
#[derive(Debug, Clone, Copy, ValueEnum)]
enum RampPreset {
    /// The default ramp
    Standard,

    /// A shorter, classic ramp
    Simple,

    /// Unicode block shading characters
    Blocks,
}

impl RampPreset {
    fn characters(self) -> &'static str {
        match self {
            RampPreset::Standard => " .-=+*x#$&X@",
            RampPreset::Simple => " .:-=+*#%@",
            RampPreset::Blocks => " ░▒▓█",
        }
    }
}

/// Validates a user-provided ramp
fn parse_ramp(s: &str) -> Result<String, String> {
    if s.chars().count() < 1 {
        return Err("a ramp needs at least 1 character".to_string());
    }
    if s.chars().any(char::is_control) {
        return Err("a ramp cannot contain control characters".to_string());
    }
    Ok(s.to_string())
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

    /// Character ramp used for the output
    #[arg(long = "ramp", value_enum, default_value_t = RampPreset::Standard, conflicts_with = "custom_ramp")]
    ramp: RampPreset,

    /// Custom character ramp, from darkest to brightest (at least 2 characters)
    #[arg(long = "custom-ramp", value_parser = parse_ramp)]
    custom_ramp: Option<String>,

    /// When to use colour in the output
    #[arg(long = "colour", value_enum, default_value_t = ColourMode::Auto)]
    colour: ColourMode,

    /// Background brightness behind each character [0.0, 1.0]. Needs coloured output
    #[arg(long = "background", default_value_t = 0.0)]
    background: f32,

    /// Edge detection threshold [0.0, 1.0]
    #[arg(short = 'e', long = "edge-threshold", default_value_t = 0.4)]
    edge_threshold: f32,
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

    /// Characters used to represent brightness, from darkest to brightest
    pub ramp: Vec<char>,

    /// Whether the output should be coloured
    pub colour: bool,

    /// Background brightness as a fraction of the pixel colour [0.0, 1.0]
    pub background: f32,

    /// Edge detection threshold [0.0, 1.0]
    pub edge_threshold: f32,
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

        let ramp: Vec<char> = match args.custom_ramp {
            Some(custom) => custom.chars().collect(),
            None => args.ramp.characters().chars().collect(),
        };

        Settings {
            file_path: args.file_path,
            max_width,
            max_height,
            char_ratio: args.char_ratio,
            resize_filter: args.resize_filter,
            ramp,
            colour,
            background: args.background.clamp(0.0, 1.0),
            edge_threshold: args.edge_threshold.clamp(0.0, 1.0),
        }
    }
}

/// Parse the CLI arguments and resolve them into Settings
pub fn parse_args() -> Settings {
    Args::parse().into()
}
