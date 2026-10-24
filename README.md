<div align="center">

# Charcoal

A cross-platform terminal-based image-to-ASCII converter written in Rust.

<img width="1024" height="768" alt="cover-photo" src="https://github.com/user-attachments/assets/87a7688d-c989-4378-9d53-0233b9ffc681" />

</div>

## Features

- Supports multiple image formats through the **image** crate
- Edge detection using the Sobel operator
- Built-in character ramps, plus support for your own
- Cross-platform coloured terminal output via the **crossterm** crate

## Installation

To install Charcoal so that it can be run from anywhere:

```
cargo install --git https://github.com/DiogoMGaspar/charcoal
```

Or, from a clone of the repository:

```
cargo install --path .
```

## How to use

Once installed, run it from anywhere:

```
charcoal <file_path> [options]
```

If you would rather not install it, you can build and run it from the project folder. Compiling the project in release mode is highly encouraged, as the **image** crate is significantly slower in debug builds. As such, you should do the following:

```
cargo build --release
./target/release/charcoal <file_path> [options]
```

To compile and run the project:

```
cargo run --release -- <file_path> [options]
```

## Options

The tool exposes several command-line options that let you control the output size, appearance, and processing behaviour. You can:

- Limit the ASCII output dimensions
- Automatically scale the image to fit your terminal
- Tune edge detection
- Adjust the character aspect ratio
- Choose the image resizing filter
- Determine whether to use colour
- Add a coloured background behind each character
- Pick a character ramp or create your own

Make sure to use the **-h** or **--help** flags for more information on how to use the tool.
