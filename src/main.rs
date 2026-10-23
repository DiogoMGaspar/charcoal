mod ascii;
mod settings;

use anyhow::Result;
use ascii::{load_image, print_image, process_image, resize_image};
use settings::parse_args;

fn main() -> Result<()> {
    let settings = parse_args();

    let image = load_image(&settings.file_path)?;

    let resized = resize_image(&image, &settings);

    let processed = process_image(&resized);

    print_image(&processed)?;

    Ok(())
}
