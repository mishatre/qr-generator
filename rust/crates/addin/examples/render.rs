//! Render a code from the command line, for looking at one.
//!
//! The component's own tests decode the pictures they render, which says whether a
//! scanner can read them. This says whether a person wants to. Use it when changing
//! the layout, the colours or the plate: a QR code that decodes and looks wrong is
//! still wrong, and no assertion in the test suite knows what "wrong" looks like.
//!
//! ```text
//! cargo run -p qr_generator_addin --example render -- "https://example.com/order/1" "AB12"
//! cargo run -p qr_generator_addin --example render -- "ORDER-1" "AB12" /tmp/code.png \
//!     '{"size": 1024, "labelSize": 0.5, "background": "#ffffff"}'
//! ```
//!
//! Arguments: `[data] [label] [output.png] [options.json]`, defaulting to a sample
//! payload, the label `AB12`, `build/qr-code.png` and the component's defaults.

use std::env;
use std::fs;
use std::process::ExitCode;

use qr_generator_addin::{render, Options};

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<String>>();

    let data = arguments
        .first()
        .cloned()
        .unwrap_or_else(|| String::from("https://example.com/orders/2026/0001?source=qr"));
    let label = arguments
        .get(1)
        .cloned()
        .unwrap_or_else(|| String::from("AB12"));
    let output = arguments
        .get(2)
        .cloned()
        .unwrap_or_else(|| String::from("build/qr-code.png"));
    let options = arguments.get(3).cloned().unwrap_or_default();

    println!("data:    {data}");
    println!("label:   {label}");
    println!(
        "options: {}",
        if options.is_empty() {
            "defaults"
        } else {
            &options
        }
    );

    let png = match render(&data, &label, &options) {
        Ok(png) => png,
        Err(error) => {
            eprintln!("the code could not be rendered: {error}");
            return ExitCode::FAILURE;
        }
    };

    // The size the options asked for, and what it cost. Printed rather than
    // summarised because the two together are the number a caller cares about: a
    // 600 pixel code that arrives as a 40 kB attachment is a different proposition
    // from one that arrives as a 3 kB attachment.
    let size = Options::from_json(&options)
        .map(|options| options.size)
        .unwrap_or(0);

    if let Some(directory) = std::path::Path::new(&output).parent() {
        if !directory.as_os_str().is_empty() {
            if let Err(error) = fs::create_dir_all(directory) {
                eprintln!("cannot create {}: {error}", directory.display());
                return ExitCode::FAILURE;
            }
        }
    }

    if let Err(error) = fs::write(&output, &png) {
        eprintln!("cannot write {output}: {error}");
        return ExitCode::FAILURE;
    }

    println!("wrote {output}: {size}x{size} pixels, {} bytes", png.len());

    ExitCode::SUCCESS
}
