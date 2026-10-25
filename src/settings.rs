use clap::{Parser, ValueEnum};
use crossterm::terminal;
use std::io::{self, IsTerminal};

/// The maximum allowed size of the ASCII output
const MAX_DIMENSION: u32 = 1024;

/// Output size used when neither the user nor the terminal provides one
const DEFAULT_WIDTH: u32 = 80;
const DEFAULT_HEIGHT: u32 = 60;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

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
    if s.is_empty() {
        return Err("a ramp needs at least 1 character".to_string());
    }
    if s.chars().any(char::is_control) {
        return Err("a ramp cannot contain control characters".to_string());
    }
    Ok(s.to_string())
}

/// Resolves the ramp to use: the custom one if provided, otherwise the preset
fn resolve_ramp(custom: Option<String>, preset: RampPreset) -> Vec<char> {
    match custom {
        Some(custom) => custom.chars().collect(),
        None => preset.characters().chars().collect(),
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColourMode {
    /// Colour only when printing to a terminal
    Auto,

    /// Always colour, even when piped or redirected
    Always,

    /// Never colour
    Never,
}

impl ColourMode {
    fn should_colour(self) -> bool {
        match self {
            ColourMode::Auto => io::stdout().is_terminal(),
            ColourMode::Always => true,
            ColourMode::Never => false,
        }
    }
}

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

    /// Custom character ramp, from darkest to brightest
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

/// Attempts to get the size of the terminal
fn terminal_size(fit_terminal: bool) -> (Option<u16>, Option<u16>) {
    fit_terminal
        .then(terminal::size)
        .and_then(Result::ok)
        .unzip()
}

/// Picks the explicit value, then the terminal value, then the default
fn pick_dimension(explicit: Option<u32>, terminal: Option<u16>, default: u32) -> u32 {
    explicit
        .or(terminal.filter(|&t| t > 0).map(u32::from))
        .unwrap_or(default)
        .min(MAX_DIMENSION)
}

impl From<Args> for Settings {
    fn from(args: Args) -> Self {
        let (terminal_width, terminal_height) = terminal_size(args.fit_terminal);

        Settings {
            file_path: args.file_path,
            max_width: pick_dimension(args.max_width, terminal_width, DEFAULT_WIDTH),
            max_height: pick_dimension(args.max_height, terminal_height, DEFAULT_HEIGHT),
            char_ratio: args.char_ratio,
            resize_filter: args.resize_filter,
            ramp: resolve_ramp(args.custom_ramp, args.ramp),
            colour: args.colour.should_colour(),
            background: args.background.clamp(0.0, 1.0),
            edge_threshold: args.edge_threshold.clamp(0.0, 1.0),
        }
    }
}

/// Parse the CLI arguments and resolve them into Settings
pub fn parse_args() -> Settings {
    Args::parse().into()
}
