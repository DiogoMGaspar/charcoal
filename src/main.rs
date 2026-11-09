mod ascii;
mod resize;
mod settings;

use anyhow::Result;
use ascii::{load_image, print_image, process_image};
use resize::resize_image;
use settings::parse_args;

fn main() -> Result<()> {
    let settings = parse_args();

    let image = load_image(&settings.file_path)?;

    let resized = resize_image(image, &settings)?;

    let processed = process_image(&resized, settings.edge_threshold, &settings.ramp);

    print_image(&processed, settings.colour, settings.background)?;

    Ok(())
}
