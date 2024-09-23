use anyhow::Context;
use chessie::Game;

/// This script exists exclusively to be used with the [perftree](https://github.com/agausmann/perftree) program for debugging.
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Print usage if insufficient arguments provided
    if args.len() < 3 {
        println!("Usage: {} <depth> <fen> [moves]", args[0]);
        std::process::exit(1);
    }

    // Parse args appropriately
    let depth = args[1]
        .parse()
        .context(format!("Failed to parse {:?} as depth value", args[1]))?;
    let mut game = Game::from_fen(&args[2])?;

    // Apply moves, if any were provided
    if args.len() > 3 {
        for mv_str in args[3].split_ascii_whitespace() {
            // Parse move string and apply it
            let mv = chessie::Move::from_uci(&game, mv_str)?;
            game.make_move(mv);
        }
    }

    // Perform a splitperft
    let nodes = perft::<true>(game, depth);

    // Print total number of nodes found
    println!("\n{nodes}");

    Ok(())
}

/// Recursive PERFT function used to validate move generation
fn perft<const SPLIT: bool>(game: Game, depth: usize) -> usize {
    // Bulk counting; if we've reached depth 1, just return the number of legal moves
    // But only if this splitperft wasn't originally called with a depth of 1.
    if !SPLIT && depth == 1 {
        return game.get_legal_moves().len();
    } else if depth == 0 {
        return 1;
    }

    // Recursively accumulate the total number of nodes
    let mut total_nodes = 0;
    for mv in game.iter() {
        // Recursive calls are not split, so pass `false`
        let nodes = perft::<false>(game.with_move_made(mv), depth - 1);

        if SPLIT {
            println!("{mv}\t{nodes}");
        }
        total_nodes += nodes;
    }

    total_nodes
}
