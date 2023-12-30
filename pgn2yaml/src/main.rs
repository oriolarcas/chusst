use chusst::{
    board::{Piece, PieceType},
    eval::move_name,
    game::Game,
};

use clap::Parser;
use regex::Regex;
use serde::{ser::SerializeMap, Serialize};
use std::{io::BufRead, path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// PGN file
    file: String,

    /// Path of the YAML file (if not specified, same as the PGN file with the extension changed)
    #[arg(short, long)]
    output: Option<String>,
}

struct Move {
    white: String,
    black: Option<String>,
}

#[derive(Default)]
struct PGN {
    moves: Vec<Move>,
    result: String,
}

enum MoveType {
    Normal,
    Capture,
    PassingPawn,
    EnPassant,
    Promotion(chusst::board::PieceType),
    KingsideCastling,
    QueensideCastling,
}

enum CheckType {
    Check,
    Checkmate,
    #[allow(dead_code)]
    Stalemate,
}

struct DetailedMoveInfo {
    mv: chusst::game::Move,
    short: String,
    long: String,
    move_type: MoveType,
    check_type: Option<CheckType>,
}

struct DetailedMove {
    white: DetailedMoveInfo,
    black: Option<DetailedMoveInfo>,
}

#[derive(Default)]
enum GameEnding {
    #[default]
    Draw,
    WhiteWinsCheckmate,
    BlackWinsCheckmate,
    WhiteResigned,
    BlackResigned,
}

#[derive(Default)]
struct DetailedGame {
    moves: Vec<DetailedMove>,
    ending: GameEnding,
}

fn parse_pgn_file(pgn_file_path: &PathBuf) -> Option<PGN> {
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
            Some(black_move_str) => pgn.moves.push(Move {
                white: white_move.to_string(),
                black: Some(black_move_str.to_string()),
            }),
            None => pgn.moves.push(Move {
                white: white_move.to_string(),
                black: None,
            }),
        }
    }

    if moves_lines.ends_with("1-0") {
        pgn.result = "1-0".to_string();
    } else if moves_lines.ends_with("0-1") {
        pgn.result = "0-1".to_string();
    } else if moves_lines.ends_with("1/2-1/2") {
        pgn.result = "1/2-1/2".to_string();
    }

    Some(pgn)
}

fn long(mv: &chusst::game::Move) -> String {
    format!("{}{}", mv.source, mv.target)
}

fn find_move_by_name(game: &Game, move_str: &str) -> DetailedMoveInfo {
    let possible_moves = chusst::eval::get_all_possible_moves(&game);
    for mv in &possible_moves {
        let mv_name = chusst::eval::move_name(&game, &mv).unwrap();

        if mv_name == move_str {
            let check_type = if move_str.contains('#') {
                Some(CheckType::Checkmate)
            } else if move_str.contains('+') {
                Some(CheckType::Check)
            } else {
                None
            };

            if let Some(index) = mv_name.find('=') {
                if let Some(promoted_piece) = mv_name.chars().nth(index + 1) {
                    return DetailedMoveInfo {
                        mv: *mv,
                        short: move_str.to_string(),
                        long: format!("{}{}", long(&mv), promoted_piece.to_lowercase()),
                        move_type: MoveType::Promotion(match promoted_piece {
                            'N' => chusst::board::PieceType::Knight,
                            'B' => chusst::board::PieceType::Bishop,
                            'R' => chusst::board::PieceType::Rook,
                            'Q' => chusst::board::PieceType::Queen,
                            _ => unreachable!(),
                        }),
                        check_type,
                    };
                }
            }

            let Some(Piece { piece, player: _ }) = game.board.square(&mv.source) else {
                panic!("Source square is empty");
            };
            let target_empty = game.board.square(&mv.target).is_none();
            let mv_rank_distance = mv.source.rank.abs_diff(mv.target.rank);
            let mv_file_distance = mv.source.file.abs_diff(mv.target.file);

            return DetailedMoveInfo {
                mv: *mv,
                short: move_str.to_string(),
                long: long(&mv),
                move_type: if piece == PieceType::Pawn && mv_rank_distance == 2 {
                    MoveType::PassingPawn
                } else if piece == PieceType::Pawn && target_empty && mv_file_distance == 1 {
                    MoveType::EnPassant
                } else if move_str == "O-O" {
                    MoveType::KingsideCastling
                } else if move_str == "O-O-O" {
                    MoveType::QueensideCastling
                } else if move_str.contains('x') {
                    MoveType::Capture
                } else {
                    MoveType::Normal
                },
                check_type,
            };
        }
    }

    panic!(
        "Move {} of player {} not found:\n{}\nPossible moves: {}",
        move_str,
        game.player,
        game.board,
        &possible_moves
            .iter()
            .map(|mv| move_name(game, mv))
            .flatten()
            .collect::<Vec<String>>()
            .join(", ")
    );
}

fn pgn_to_long_algebraic(pgn: &PGN) -> DetailedGame {
    let mut game = Game::new();
    let mut detailed = DetailedGame::default();

    let mut checkmate = false;

    for mv in pgn.moves.iter() {
        let white_short_str = mv.white.as_str();

        let detailed_white_mv = find_move_by_name(&game, white_short_str);

        checkmate = white_short_str.contains('#');

        chusst::eval::do_move(&mut game, &detailed_white_mv.mv);

        if let Some(black_short_str) = mv.black.as_deref() {
            let detailed_black_mv = find_move_by_name(&game, black_short_str);
            chusst::eval::do_move(&mut game, &detailed_black_mv.mv);

            detailed.moves.push(DetailedMove {
                white: detailed_white_mv,
                black: Some(detailed_black_mv),
            });

            checkmate = black_short_str.contains('#');
        } else {
            detailed.moves.push(DetailedMove {
                white: detailed_white_mv,
                black: None,
            });
        }
    }

    detailed.ending = if pgn.result.ends_with("1-0") {
        if checkmate {
            GameEnding::WhiteWinsCheckmate
        } else {
            GameEnding::BlackResigned
        }
    } else if pgn.result.ends_with("0-1") {
        if checkmate {
            GameEnding::BlackWinsCheckmate
        } else {
            GameEnding::WhiteResigned
        }
    } else if pgn.result.ends_with("1/2-1/2") {
        GameEnding::Draw
    } else {
        panic!("Unknown ending");
    };
    detailed
}

impl Serialize for DetailedMoveInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map =
            serializer.serialize_map(Some(if self.check_type.is_some() { 4 } else { 3 }))?;

        map.serialize_entry("short", &self.short)?;
        map.serialize_entry("long", &self.long)?;
        map.serialize_entry(
            "type",
            match &self.move_type {
                MoveType::Normal => "normal",
                MoveType::Capture => "capture",
                MoveType::PassingPawn => "passing pawn",
                MoveType::EnPassant => "en passant",
                MoveType::Promotion(piece) => match piece {
                    PieceType::Knight => "promotion to knight",
                    PieceType::Bishop => "promotion to bishop",
                    PieceType::Rook => "promotion to rook",
                    PieceType::Queen => "promotion to queen",
                    _ => unreachable!(),
                },
                MoveType::KingsideCastling => "kingside castling",
                MoveType::QueensideCastling => "queenside castling",
            },
        )?;

        if let Some(check_type) = &self.check_type {
            map.serialize_entry(
                "check",
                match check_type {
                    CheckType::Check => "check",
                    CheckType::Checkmate => "checkmate",
                    CheckType::Stalemate => "stalemate",
                },
            )?;
        }

        map.end()
    }
}

impl Serialize for DetailedMove {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(black) = &self.black {
            let mut map = serializer.serialize_map(Some(2))?;

            map.serialize_entry("white", &self.white)?;
            map.serialize_entry("black", &black)?;

            map.end()
        } else {
            let mut map = serializer.serialize_map(Some(1))?;

            map.serialize_entry("white", &self.white)?;

            map.end()
        }
    }
}

impl Serialize for DetailedGame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        map.serialize_entry(
            "result",
            match self.ending {
                GameEnding::Draw => "Draw",
                GameEnding::WhiteWinsCheckmate => "White wins: checkmate",
                GameEnding::BlackWinsCheckmate => "Black wins: checkmate",
                GameEnding::WhiteResigned => "Black wins: white resigned",
                GameEnding::BlackResigned => "White wins: black resigned",
            },
        )?;
        map.serialize_entry("moves", &self.moves)?;

        map.end()
    }
}

fn write_yaml(yaml_path: &PathBuf, game: &DetailedGame) -> Result<(), String> {
    let output = std::fs::File::create(yaml_path).map_err(|_| {
        format!(
            "Could not open file {} for writing",
            yaml_path.to_string_lossy()
        )
    })?;

    serde_yaml::to_writer(output, game)
        .map_err(|_| format!("Error writing YAML data to {}", yaml_path.to_string_lossy()))
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let pgn_path = PathBuf::from(cli.file);

    let pgn = parse_pgn_file(&pgn_path).unwrap();

    let detailed_game = pgn_to_long_algebraic(&pgn);

    for (index, mv) in detailed_game.moves.iter().enumerate() {
        if let Some(black_mv) = &mv.black {
            println!("{}. {} {}", index + 1, mv.white.long, black_mv.long);
        } else {
            println!("{}. {}", index + 1, mv.white.long);
        }
    }
    match detailed_game.ending {
        GameEnding::Draw => println!("Draw"),
        GameEnding::WhiteWinsCheckmate => println!("White wins: checkmate"),
        GameEnding::BlackWinsCheckmate => println!("Black wins: checkmate"),
        GameEnding::WhiteResigned => println!("Black wins: white resigned"),
        GameEnding::BlackResigned => println!("White wins: black resigned"),
    }

    let yaml_path = cli.output.map_or(
        {
            let mut path = pgn_path;
            path.set_extension("png");
            path
        },
        PathBuf::from,
    );
    if let Err(reason) = write_yaml(&yaml_path, &detailed_game) {
        println!("{}", reason);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
