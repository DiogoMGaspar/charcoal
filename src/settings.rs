use clap::{Parser, ValueEnum};
use crossterm::terminal;
use std::io::{self, IsTerminal};

const MAX_DIMENSION: u32 = 1024;

const DEFAULT_WIDTH: u32 = 80;
const DEFAULT_HEIGHT: u32 = 60;

#[derive(Clone, Copy, ValueEnum)]
pub enum ResizeFilter {
    Nearest,
    Bilinear,
    Hamming,
    CatmullRom,
    Mitchell,
    Gaussian,
    Lanczos3,
}

#[derive(Clone, Copy, ValueEnum)]
enum RampPreset {
    /// The default ramp
    Standard,

    /// A shorter ramp
    Simple,

    /// Unicode block shading characters
    Blocks,
}

impl RampPreset {
    fn characters(self) -> &'static str {
        match self {
            RampPreset::Standard => " .-=+*x#$&X@",
            RampPreset::Simple => " .:=+*#%@",
            RampPreset::Blocks => " ░▒▓█",
        }
    }
}

fn parse_ramp(s: &str) -> Result<String, String> {
    if s.is_empty() {
        return Err("a ramp needs at least 1 character".to_string());
    }
    if s.chars().any(char::is_control) {
        return Err("a ramp cannot contain control characters".to_string());
    }
    Ok(s.to_string())
}

fn resolve_ramp(custom: Option<String>, preset: RampPreset) -> Vec<char> {
    match custom {
        Some(custom) => custom.chars().collect(),
        None => preset.characters().chars().collect(),
    }
}

#[derive(Clone, Copy, ValueEnum)]
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

fn parse_positive(string: &str) -> Result<f32, String> {
    let value: f32 = string.parse().map_err(|_| "not a number".to_string())?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err("must be a positive, finite number".to_string())
    }
}

#[derive(Parser)]
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

    /// Try to scale the image so as to fit the terminal's dimensions
    #[arg(long = "fit-terminal")]
    fit_terminal: bool,

    /// Character aspect ratio
    #[arg(short = 'c', long = "char-ratio", default_value_t = 2.0, value_parser = parse_positive, allow_negative_numbers = true)]
    char_ratio: f32,

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

    /// Gamma correction to control the luminance of the image
    #[arg(long = "gamma", default_value_t = 1.0, value_parser = parse_positive, allow_negative_numbers = true)]
    gamma: f32,

    /// Background brightness behind each character [0.0, 1.0]. Needs coloured output
    #[arg(long = "background", default_value_t = 0.0)]
    background: f32,

    /// Edge detection threshold [0.0, 1.0]
    #[arg(short = 'e', long = "edge-threshold", default_value_t = 0.65)]
    edge_threshold: f32,
}

pub struct Settings {
    pub file_path: String,
    pub max_width: u32,
    pub max_height: u32,
    pub char_ratio: f32,
    pub resize_filter: ResizeFilter,
    pub ramp: Vec<char>,
    pub colour: bool,
    pub gamma: f32,
    pub background: f32,
    pub edge_threshold: f32,
}

fn terminal_size(fit_terminal: bool) -> (Option<u16>, Option<u16>) {
    fit_terminal
        .then(terminal::size)
        .and_then(Result::ok)
        .unzip()
}

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
            gamma: args.gamma,
            background: args.background.clamp(0.0, 1.0),
            edge_threshold: args.edge_threshold.clamp(0.0, 1.0),
        }
    }
}

pub fn parse_args() -> Settings {
    Args::parse().into()
}
