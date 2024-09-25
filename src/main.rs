/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use autoperft::PerftChecker;
use clap::Parser;

/// Command-line tool for debugging chess move generation
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the script to run your move generator.
    #[arg(id = "path/to/user/script")]
    movegen_script: String,

    /// Path to the EPD file with which to test your move generator.
    #[arg(
        id = "EPD file path",
        short = 'e',
        long = "epd",
        default_value = "src/standard.epd"
    )]
    epd_file_path: String,
}

fn main() {
    let args = Args::parse();

    let checker = PerftChecker::new(&args.movegen_script);

    if let Err(e) = checker.run(&args.epd_file_path) {
        println!(
            "\n{} failed with the following error:\n{e}",
            env!("CARGO_PKG_NAME")
        );
    }
}
