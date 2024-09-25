# AutoPerft

A simple CLI tool to help debug your chess engine's move generator.

Inspired by [`perftree`](https://github.com/agausmann/perftree).

## Overview

When writing a chess engine (the move generation part of it, anyway), the standard for ensuring that your engine is generating legal moves is to test it through a [perft](https://www.chessprogramming.org/Perft) function.
This involves starting with a position and, for every legal move available, playing that move on the position and recursing until a depth limit is reached.

However, perft results just tell you if your engine generates the correct number of moves.
They don't tell you which move(s) your engine generated incorrectly, or which position(s) are troublesome for your engine.
In order to find these, you must manually create and walk a perft tree until you find the problematic position(s).

That's a lot of work, and while tools like [`perftree`](https://github.com/agausmann/perftree) and [`webperft`](https://analog-hors.github.io/webperft/) are very helpful, I wanted something a bit more automated.
AutoPerft automates the process of debugging your move generator by giving a position to your move generator, checking its perft results and, if they're incorrect, recursively making moves and checking perft results until the exact position that your generator fails on is found.
Additionally, AutoPerft outputs the list of moves that were applied to the original position, so that you can debug your engine's behavior on lines rather than static positions.
This is useful, for example, if your engine doesn't properly update castling rights.
Ask me how I know...

## Installation

This crate is not yet on [`crates.io`](https://crates.io/), so you must download and build from source:

0. Ensure you have [`rust`](https://www.rust-lang.org/)
1. Clone this repository `git clone https://github.com/dannyhammer/autoperft`

This project internally uses the [`chess`](https://crates.io/crates/chess) crate for validation and runs on a [suite of 128 positions](https://github.com/dannyhammer/autoperft/blob/main/src/standard.epd), so no additional dependencies are required.

## Usage

AutoPerft needs some way of interacting with your move generator.
To accomplish this, you must provide a script that executes a splitperft on an arbitrary position.
A "splitperft" is the same as a normal perft, except that you count the number of nodes reachable from every legal moves on the current position, as well.

### User script

The script will be called as follows:

```bash
./your-script <depth> <fen> [moves]
```

where:

-   `<depth>` is the depth to run the perft
-   `<fen>` is the [FEN string](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation) of the position to search
-   `[moves]` (optional) is a list of moves in [UCI](https://en.wikipedia.org/wiki/Universal_Chess_Interface#Design) notation (`<start_square><end_square>[promotion]`) that will be applied to `<fen>`.

Your script's output should be formatted like so:

```bash
<move_1> <nodes_reachable_from_move_1>
<move_2> <nodes_reachable_from_move_2>
...
<move_n> <nodes_reachable_from_move_n>

<total_nodes_reachable_from_position>
```

As an example, here is what your script should output on a depth 3 perft from the starting position, after making the moves `e2e4`, `e7e5`:

```bash
$ ./your-script 2 "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" "e2e4" "e7e5"
a7a5    30
a7a6    30
b7b5    29
b7b6    30
c7c5    30
c7c6    30
d7d5    31
d7d6    30
e7e5    29
e7e6    30
f7f5    31
f7f6    30
g7g5    30
g7g6    30
h7h5    30
h7h6    30
b8a6    30
b8c6    30
g8f6    30
g8h6    30

600
```

Please ensure that your script handles the inputs correctly (note the quotes around the FEN and each individual move) and prints it's output to `stdout` properly.
There must be at least one character of whitespace between a move and it's nodes, and you must have a blank line before the total node count.

For an example script, please see the [`examples/`](https://github.com/dannyhammer/autoperft/tree/main/examples) directory:

-   [`perftree_script.rs`](https://github.com/dannyhammer/autoperft/blob/main/examples/perftree_script.rs) shows an example Rust program to execute a splitperft.
-   [`run-perftree-script.sh`](https://github.com/dannyhammer/autoperft/blob/main/examples/run-perftree-script.sh) just executes `perftree_script.rs`.

### Launching the debugger

All that's left is to launch `autoperft` from the command line and pass in your move generator's script:

```bash
cargo run --release ./your-script
```

Additionally, there are some command line flags that can be set:

-   `-e <file>` Specify a `.epd` file to test with, instead of using the [provided suite](https://github.com/dannyhammer/autoperft/blob/main/src/standard.epd)
-   `-s <n>` Skip the first `n` tests in the `.epd` file (either the standard file or user-supplied)
-   `-f <n>` Run only the first `n` tests in the `.epd` file (either the standard file or user-supplied)

Run `--help` for more information.

## Contributing

There are some very impressive projects in the world of chess programming.
This? This is not one of them.

I am sure there is plenty of room for improvement, and I myself intend to refactor this project over time.
However, if you notice any bugs while using AutoPerft, have suggestions about new features, or know of some way to speed things up, I'd love to hear about it.
Please don't hesitate to [create an issue](https://github.com/dannyhammer/autoperft/issues).
