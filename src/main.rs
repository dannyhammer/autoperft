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
    #[arg(short = 'e', long = "epd", default_value = "src/standard.epd")]
    epd_file_path: String,

    /// Skip the first N test suites
    #[arg(short = 's', long = "skip", default_value = "0")]
    skip: usize,

    /// Only run the first N test suites.
    ///
    /// If `--skip` was provided, this value is factored in BEFORE skipping `--skip` tests.
    /// i.e. `--skip 10 --first 13` will run tests 10, 11, and 12.
    #[arg(short = 'f', long = "first", default_value = "128")]
    first: usize,
}

fn main() {
    let args = Args::parse();

    let checker = PerftChecker::new(&args.movegen_script);

    // Ensure indices are proper
    if args.skip >= args.first {
        println!(
            "Argument for `--skip` ({}) must be strictly less than argument for `--first` ({})",
            args.skip, args.first
        );
        std::process::exit(1);
    }

    // Run the checker on the test suite file
    if let Err(e) = checker.run(&args.epd_file_path, args.skip, args.first) {
        println!(
            "\n{} failed with the following error:\n{e}",
            env!("CARGO_PKG_NAME")
        );
    }
}
