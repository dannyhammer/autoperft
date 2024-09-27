/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{cmp::Ordering, path::Path, process::Command};

use anyhow::{anyhow, bail, Context, Result};
use chessie::{perft, Game, Move};

/// Encapsulates frequently-used data like the user-supplied script
pub struct PerftChecker<'a> {
    user_script: &'a str,
}

impl<'a> PerftChecker<'a> {
    /// Create the checker with the user-supplied script
    pub fn new(user_script: &'a str) -> Self {
        Self { user_script }
    }

    /// Runs the checker on the provided EPD file.
    pub fn run(
        &self,
        epd_file: impl AsRef<Path>,
        start_index: usize,
        end_index: usize,
    ) -> Result<()> {
        let contents = std::fs::read_to_string(epd_file)?;
        // TODO: Is there a way to iterate over only a subset of the whole WITHOUT collecting it first?
        // Also, can we get the total number of lines WITHOUT having to collect it?
        let epd_tests = Vec::from(&contents.lines().collect::<Vec<_>>()[start_index..end_index]);
        let num_tests = epd_tests.len();

        // Run each individual test suite
        for (i, epd) in epd_tests.into_iter().enumerate() {
            let (fen, tests) = self.parse_epd(epd)?;
            println!(
                "Beginning tests on perft suite {}/{num_tests}: {fen:?}",
                i + 1
            );
            self.check_epd(fen, tests)?;
        }

        Ok(())
    }

    /// Checks that all of the PERFT results on the provided `epd` string are valid.
    fn check_epd(&self, fen: &str, tests: Vec<(usize, u64)>) -> Result<()> {
        for (depth, expected) in tests {
            println!("\tChecking perft({depth}) := {expected}");
            // Check if the user-supplied move generator is correct for this depth and FEN
            self.check_splitperft::<false>(depth, fen, &[])?;
        }

        Ok(())
    }

    /// Executes the user-supplied splitperft script, returning it's `stdout`.
    fn exec_user_perft(&self, depth: usize, fen: &str, moves: &[String]) -> Result<String> {
        // Create the command
        let mut cmd = Command::new(self.user_script);

        // Execute the command, saving its output
        let output = cmd
            .arg(depth.to_string())
            .arg(fen)
            .arg(moves.join(" "))
            .output()
            .context(format!(
                "Failed to execute user splitperft script:\n{cmd:?}"
            ))?;

        // If execution failed, print the process' stderr
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)
                .context("Failed to convert stderr of child process to String")?;

            bail!(
                "{} crashed on: {fen:?}\n\nFull error:\n{stderr}",
                self.user_script
            );
        }

        // Extract and return the stdout from the script
        let stdout = String::from_utf8(output.stdout)
            .context("Failed to convert stdout of child process to String")?;

        Ok(stdout)
    }

    /// Parses the output of [`Self::exec_user_perft`] to return the splitperft results.
    fn parse_splitperft_results(&self, stdout: &str) -> Result<Vec<(String, u64)>> {
        // Parse the splitperft results
        let results = stdout
            .lines()
            .filter_map(|line| {
                // Split the move and node count
                let mut split = line.split_ascii_whitespace();

                // If `next()` fails, we've reached the end of the splitperft results
                let mv = split.next()?.trim().to_string();
                let nodes = split
                    .next()?
                    .trim()
                    .parse()
                    .expect("Failed to parse node count");

                Some((mv, nodes))
            })
            .collect();

        Ok(results)
    }

    /// Parses the output of [`Self::exec_user_perft`] to return the total node count.
    fn parse_splitperft_output_nodes_only(&self, stdout: &str) -> Result<u64> {
        let nodes = stdout.lines().last().ok_or(anyhow!(
            "User script must have a final line containing total number of nodes"
        ))?;
        let nodes = nodes
            .parse()
            .context("Failed to parse final line of user script output")?;

        Ok(nodes)
    }

    /// Generates a (correct) splitperft.
    ///
    /// For each legal move, it generates the possible nodes reachable from playing that move.
    fn generate_splitperft(
        &self,
        depth: usize,
        fen: &str,
        moves: &[String],
    ) -> (Vec<(String, u64)>, u64) {
        let mut results = Vec::with_capacity(128);
        let mut board = Game::from_fen(fen).unwrap();

        // If there were any moves supplied, apply them
        for mv_str in moves {
            match Move::from_uci(&board, mv_str) {
                Ok(mv) => board = board.with_move_made(mv),
                Err(_) => panic!("Invalid move {mv_str} for position {board}"),
            };
        }

        let mut nodes = 0;
        for mv in board.get_legal_moves() {
            let new_board = board.with_move_made(mv);

            let new_nodes = perft(&new_board, depth - 1);
            nodes += new_nodes;

            results.push((mv.to_string(), new_nodes));
        }

        (results, nodes)
    }

    /// Parses an EPD string into its components.
    fn parse_epd(&self, epd: &'a str) -> Result<(&'a str, Vec<(usize, u64)>)> {
        let mut tests = Vec::with_capacity(8);

        // Split the EPD string into its components
        let mut parts = epd.split(';');

        // Extract the FEN
        let fen = parts
            .next()
            .context(format!("Missing FEN in {epd:?}"))?
            .trim();

        // Extract the depth and expected node counts for the remaining parts of the EPD string
        for perft_data in parts {
            // Extract and parse the depth
            let depth = perft_data
                .get(1..2)
                .context(format!("Missing depth value in {perft_data:?}"))?
                .trim();
            let depth = depth
                .parse()
                .context(format!("Invalid depth value {depth:?}"))?;

            // Extract and parse the expected nodes
            let expected = perft_data
                .get(3..)
                .context(format!("Missing expected nodes value in {perft_data:?}"))?
                .trim();
            let expected = expected
                .parse()
                .context(format!("Invalid expected nodes value {expected:?}"))?;

            tests.push((depth, expected));
        }

        Ok((fen, tests))
    }

    /// Check if the user-supplied script generated the correct splitperft results on the provided position.
    ///
    /// If not, recursive down the line of illegal moves until the "problematic" position is found.
    /// Once found, return an error explaining what the user-supplied script is doing wrong.
    fn check_splitperft<const ILLEGAL: bool>(
        &self,
        depth: usize,
        fen: &str,
        moves: &[String],
    ) -> Result<()> {
        // Get the perft results from the user-supplied script
        let user_output = self.exec_user_perft(depth, fen, moves)?;
        // We only need the total node count for now
        let user_nodes = self.parse_splitperft_output_nodes_only(&user_output)?;

        // Generate the correct result
        let (correct_splitperft, correct_nodes) = self.generate_splitperft(depth, fen, moves);

        // If we've reached depth 1 in an illegal line, we need to find which move(s) are the problematic ones
        if ILLEGAL && depth == 1 {
            let user_splitperft = self.parse_splitperft_results(&user_output)?;

            // Fetch all of the legal moves for this position
            let mut legal_moves = correct_splitperft
                .into_iter()
                .map(|(mv, _)| mv)
                .collect::<Vec<_>>();
            legal_moves.sort();

            // Fetch all of the moves the user's script created for this position
            let mut generated_moves = user_splitperft
                .into_iter()
                .map(|(mv, _)| mv)
                .collect::<Vec<_>>();
            generated_moves.sort();

            // Get the reason as to why these lists are different.
            let Err(list_diff_err) = check_move_lists(legal_moves, generated_moves) else {
                unreachable!(
                    "Since we're in an illegal line, the provided lists should never be equal"
                );
            };

            // Generate the FEN of the position after applying all of the problematic moves
            let new_fen = generate_fen_from(fen, moves);

            // Format and return the error message
            let moves_str = moves.join(", ");
            bail!("{list_diff_err}\nApplied moves: {moves_str}\nResulting FEN: {new_fen:?}");
        }

        // If the user-supplied script did not generate the proper number of nodes, there's an error we need to find
        if user_nodes != correct_nodes {
            eprint!("\tUser script generated {user_nodes} nodes",);
            if !moves.is_empty() {
                eprintln!("after applying {} to {fen:?}", moves.join(" "));
            } else {
                eprintln!();
            }

            let user_splitperft = self.parse_splitperft_results(&user_output)?;

            // To start, we need to find which move in the splitperft leads to an incorrect number of nodes
            for (mv, correct_nodes_for_mv) in correct_splitperft {
                // Make sure the user-supplied script generated this (correct) move, erroring out if it didn't
                let Some((_, user_nodes_for_mv)) =
                    user_splitperft.iter().find(|(user_mv, _)| *user_mv == mv)
                else {
                    // Generate the FEN of the position after applying all of the problematic moves
                    let new_fen = generate_fen_from(fen, moves);

                    // Format and return the error
                    let moves_str = moves.join(", ");
                    let user_moves = user_splitperft
                        .into_iter()
                        .map(|(mv, _)| mv)
                        .collect::<Vec<_>>()
                        .join(" ");
                    bail!("Failed to generate legal move {mv:?}\nApplied moves: {moves_str}\nResulting FEN: {new_fen:?}\nGenerated moves: {user_moves}");
                };

                // If the user-supplied script generated an incorrect number of nodes after this move, then we need to follow this move until we reach the problematic position
                if *user_nodes_for_mv != correct_nodes_for_mv {
                    eprintln!("Move {mv:?} at depth {depth} on {fen:?} yields an incorrect node count ({user_nodes_for_mv}). Correct: {correct_nodes_for_mv}");

                    // Track this move
                    let mut moves_to_inspect = Vec::from(moves);
                    moves_to_inspect.push(mv);

                    // Recursively check the resulting position
                    self.check_splitperft::<true>(depth - 1, fen, &moves_to_inspect)?;
                }
            }
        }

        Ok(())
    }
}

/// Generates a FEN string after applying all of `moves` to the provided `fen`.
fn generate_fen_from(fen: &str, moves: &[String]) -> String {
    // eprintln!("Generating FEN from {moves:?} on {fen:?}");
    let mut board = Game::from_fen(fen).unwrap();

    for mv_str in moves {
        let mv = Move::from_uci(&board, mv_str).unwrap();
        // TODO: Maybe call `is_legal`?
        board = board.with_move_made(mv);
    }

    board.to_string()
}

/// Checks the contents of two lists.
///
/// The length of the lists is checked first.
/// If the lengths do not match, either you have generated an illegal move, or failed to generate a legal one.
/// If both lists are of equal length, [`check_move_lists_of_equal_length`] is called.
fn check_move_lists(mut expected: Vec<String>, mut generated: Vec<String>) -> Result<()> {
    match expected.len().cmp(&generated.len()) {
        // If there are more moves in the expected list, the supplied move generator failed to generate some legal moves
        Ordering::Greater => {
            expected.retain(|mv| !generated.contains(mv));
            let word = if expected.len() > 1 { "moves" } else { "move" };
            bail!("Failed to generate legal {word}: {}", expected.join(", "));
        }

        // If there are more moves in the supplied list, the supplied move generator generated illegal moves
        Ordering::Less => {
            generated.retain(|mv| !expected.contains(mv));
            let word = if generated.len() > 1 { "moves" } else { "move" };
            bail!("Illegal {word} generated: {}", generated.join(", "));
        }

        // If the lengths of both lists match, we need to check that each move generated is correct.
        Ordering::Equal => check_move_lists_of_equal_length(expected, generated),
    }
}

/// Checks the contents of two lists of equal length.
///
/// If `generated` contains a move that `expected` does not, this returns an error.
fn check_move_lists_of_equal_length(expected: Vec<String>, generated: Vec<String>) -> Result<()> {
    // Create a list of all moves that are in `generated` and NOT in `expected`.
    let illegal = generated
        .iter()
        .filter(|mv| !expected.contains(mv))
        .collect::<Vec<_>>();

    // If there are any such moves, they are illegal
    if !illegal.is_empty() {
        let word = if illegal.len() > 1 { "moves" } else { "move" };
        bail!(
            "Generated illegal {word}: {}\nAnd neglected to generate: {}",
            generated.join(", "),
            expected.join(", ")
        )
    }

    // If there are no such moves, we're good to go!
    Ok(())
}
