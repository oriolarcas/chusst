use chusst::game::Game;

use clap::Parser;
use regex::Regex;
use std::io::BufRead;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// PGN file
    file: String,
}

struct FullMove {
    white: String,
    black: String,
}

struct HalfMove {
    white: String,
}

enum Move {
    Full(FullMove),
    Half(HalfMove),
}

#[derive(Default)]
struct PGN {
    moves: Vec<Move>,
    result: String,
}

fn parse_pgn_file(pgn_file_path: String) -> Option<PGN> {
    let mut pgn = PGN::default();

    let file = std::fs::File::open(pgn_file_path).ok()?;
    let lines = std::io::BufReader::new(file).lines();

    // Extract only the moves data
    let mut moves_lines = String::new();

    for line_result in lines {
        let Ok(line_content) = line_result else {
            return None;
        };

        let line = line_content.trim();

        if line.is_empty() {
            continue;
        }

        // Ignore tags
        if line.starts_with('[') {
            continue;
        }

        if !moves_lines.is_empty() && !moves_lines.ends_with(' ') {
            moves_lines.push(' ');
        }

        moves_lines.push_str(line);
    }

    // println!("<<{}>>", moves_lines);

    // Parse the moves
    let move_re = Regex::new(r"(\d+)\. (\S+) (\S+)?").unwrap();
    for full_move_match in move_re.captures_iter(&moves_lines) {
        let move_number = full_move_match.get(1)?.as_str().parse::<u32>().ok()?;
        assert_eq!(
            move_number as usize,
            pgn.moves.len() + 1,
            "Invalid move number"
        );

        let white_move = full_move_match.get(2)?.as_str();
        let black_move = if let Some(black_move) = full_move_match.get(3) {
            match black_move.as_str() {
                result_str @ "1-0" | result_str @ "0-1" | result_str @ "1/2-1/2" => {
                    pgn.result = result_str.to_string();
                    None
                }
                "*" => panic!("Unfinished game"),
                move_str => Some(move_str),
            }
        } else {
            None
        };

        match black_move {
            Some(black_move_str) => pgn.moves.push(Move::Full(FullMove {
                white: white_move.to_string(),
                black: black_move_str.to_string(),
            })),
            None => pgn.moves.push(Move::Half(HalfMove {
                white: white_move.to_string(),
            })),
        }
    }

    Some(pgn)
}

fn long(mv: &chusst::game::Move) -> String {
    format!("{}{}", mv.source, mv.target)
}

fn find_move_by_name(game: &Game, move_str: &str) -> (chusst::game::Move, String) {
    for mv in chusst::eval::get_all_possible_moves(&game) {
        let mv_name = chusst::eval::move_name(&game, &mv).unwrap();

        if mv_name == move_str {
            if let Some(index) = mv_name.find('=') {
                if let Some(promoted_piece) = mv_name.chars().nth(index + 1) {
                    return (mv, format!("{}{}", long(&mv), promoted_piece.to_lowercase()));
                }
            }

            return (mv, long(&mv));
        }
    }

    panic!(
        "Move {} of player {} not found:\n{}\n",
        move_str, game.player, game.board
    );
}

fn pgn_to_long_algebraic(pgn: &PGN) {
    let mut game = Game::new();

    for (index, mv) in pgn.moves.iter().enumerate() {
        let white_move_str = match mv {
            Move::Full(full_move) => full_move.white.as_str(),
            Move::Half(half_move) => half_move.white.as_str(),
        };

        let (white_mv, white_mv_str) = find_move_by_name(&game, white_move_str);

        chusst::eval::do_move(&mut game, &white_mv);

        if let Move::Full(full_move) = mv {
            let (black_mv, black_mv_str) = find_move_by_name(&game, &full_move.black);
            chusst::eval::do_move(&mut game, &black_mv);

            println!("{}. {} {}", index + 1, white_mv_str, black_mv_str);
        } else {
            println!("{}. {}", index + 1, white_mv_str);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let pgn = parse_pgn_file(cli.file).unwrap();

    // for (index, mv) in pgn.moves.iter().enumerate() {
    //     match mv {
    //         Move::Full(full_move) => {
    //             println!("{}. {} {}", index + 1, full_move.white, full_move.black);
    //         }
    //         Move::Half(half_move) => {
    //             println!("{}. {}", index + 1, half_move.white);
    //         }
    //     }
    // }
    // println!("{}", pgn.result);

    pgn_to_long_algebraic(&pgn);
}
