mod reader;

use chusst::{
    board::{Piece, PieceType},
    eval::move_name,
    game::Game,
};
use reader::parser::Tag;
use reader::parser::PGN;

use anyhow::{bail, Ok};
use anyhow::{Context, Result};
use clap::Parser;
use serde::{ser::SerializeMap, Serialize};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// PGN file
    file: String,

    /// Path of the YAML file (if not specified, same as the PGN file with the extension changed)
    #[arg(short, long)]
    output: Option<String>,
}

#[derive(PartialEq)]
enum PromotionPieces {
    Knight,
    Bishop,
    Rook,
    Queen,
}

#[derive(PartialEq)]
enum MoveType {
    Normal,
    Capture,
    PassingPawn,
    EnPassant,
    Promotion(PromotionPieces),
    PromotionWithCapture(PromotionPieces),
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

#[derive(Default, Clone, Copy)]
enum GameEnding {
    #[default]
    Draw,
    Stalemate,
    WhiteWinsCheckmate,
    BlackWinsCheckmate,
    WhiteResigned,
    BlackResigned,
}

#[derive(Default)]
struct DetailedGame {
    tags: Vec<Tag>,
    moves: Vec<DetailedMove>,
    ending: GameEnding,
}

fn parse_pgn_file(pgn_file_path: &PathBuf) -> Result<Vec<PGN>> {
    reader::parser::Parser::parse_file(pgn_file_path)
}

fn long(mv: &chusst::game::Move, capture: bool) -> String {
    format!(
        "{}{}{}",
        mv.source,
        if capture { "x" } else { "-" },
        mv.target
    )
}

fn find_move_by_name(game: &Game, move_str: &str) -> Result<DetailedMoveInfo> {
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

            let is_capture = move_str.contains('x');

            if let Some(index) = mv_name.find('=') {
                if let Some(promoted_piece_char) = mv_name.chars().nth(index + 1) {
                    let promoted_piece = match promoted_piece_char {
                        'N' => PromotionPieces::Knight,
                        'B' => PromotionPieces::Bishop,
                        'R' => PromotionPieces::Rook,
                        'Q' => PromotionPieces::Queen,
                        _ => unreachable!(),
                    };
                    let move_type = if is_capture {
                        MoveType::PromotionWithCapture(promoted_piece)
                    } else {
                        MoveType::Promotion(promoted_piece)
                    };
                    return Ok(DetailedMoveInfo {
                        mv: *mv,
                        short: move_str.to_string(),
                        long: format!(
                            "{}{}",
                            long(&mv, is_capture),
                            promoted_piece_char.to_lowercase()
                        ),
                        move_type,
                        check_type,
                    });
                }
            }

            let Some(Piece { piece, player: _ }) = game.board.square(&mv.source) else {
                bail!("Source square is empty");
            };
            let target_empty = game.board.square(&mv.target).is_none();
            let mv_rank_distance = mv.source.rank.abs_diff(mv.target.rank);
            let mv_file_distance = mv.source.file.abs_diff(mv.target.file);

            return Ok(DetailedMoveInfo {
                mv: *mv,
                short: move_str.to_string(),
                long: long(&mv, is_capture),
                move_type: if piece == PieceType::Pawn && mv_rank_distance == 2 {
                    MoveType::PassingPawn
                } else if piece == PieceType::Pawn && target_empty && mv_file_distance == 1 {
                    MoveType::EnPassant
                } else if move_str == "O-O" {
                    MoveType::KingsideCastling
                } else if move_str == "O-O-O" {
                    MoveType::QueensideCastling
                } else if is_capture {
                    MoveType::Capture
                } else {
                    MoveType::Normal
                },
                check_type,
            });
        }
    }

    bail!(
        "Invalid notation or ilegal move {} of player {}:\n{}\nPossible moves: {}",
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

fn is_stalemate(game: &Game) -> bool {
    chusst::eval::get_all_possible_moves(&game).is_empty()
}

fn pgn_to_long_algebraic(pgn: &PGN) -> Result<DetailedGame> {
    let mut game = Game::new();
    let mut detailed = DetailedGame {
        tags: pgn.tags.clone(),
        moves: Default::default(),
        ending: Default::default(),
    };

    let mut checkmate = false;

    for mv in pgn.moves.iter() {
        let white_short_str = mv.white.as_str();

        let detailed_white_mv = find_move_by_name(&game, white_short_str)?;

        checkmate = white_short_str.contains('#');

        chusst::eval::do_move(&mut game, &detailed_white_mv.mv);

        if let Some(black_short_str) = mv.black.as_deref() {
            let detailed_black_mv = find_move_by_name(&game, black_short_str)?;
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
        if is_stalemate(&game) {
            GameEnding::Stalemate
        } else {
            GameEnding::Draw
        }
    } else {
        bail!("Unexpected ending value");
    };
    Ok(detailed)
}

impl Serialize for DetailedMoveInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut entries = 2;
        if self.move_type != MoveType::Normal {
            entries += 1;
        }
        if self.check_type.is_some() {
            entries += 1;
        }

        let mut map = serializer.serialize_map(Some(entries))?;

        map.serialize_entry("short", &self.short)?;
        map.serialize_entry("long", &self.long)?;
        if self.move_type != MoveType::Normal {
            map.serialize_entry(
                "type",
                match &self.move_type {
                    MoveType::Normal => unreachable!(),
                    MoveType::Capture => "capture",
                    MoveType::PassingPawn => "passing pawn",
                    MoveType::EnPassant => "en passant",
                    MoveType::Promotion(piece) => match piece {
                        PromotionPieces::Knight => "promotion to knight",
                        PromotionPieces::Bishop => "promotion to bishop",
                        PromotionPieces::Rook => "promotion to rook",
                        PromotionPieces::Queen => "promotion to queen",
                    },
                    MoveType::PromotionWithCapture(piece) => match piece {
                        PromotionPieces::Knight => "promotion to knight with capture",
                        PromotionPieces::Bishop => "promotion to bishop with capture",
                        PromotionPieces::Rook => "promotion to rook with capture",
                        PromotionPieces::Queen => "promotion to queen with capture",
                    },
                    MoveType::KingsideCastling => "kingside castling",
                    MoveType::QueensideCastling => "queenside castling",
                },
            )?;
        }

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

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.key, &self.value)?;
        map.end()
    }
}

struct SerializedMoveList<'a>(&'a Vec<DetailedMove>);

impl<'a> Serialize for SerializedMoveList<'a> {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (index, mv) in self.0.iter().enumerate() {
            let move_number = index + 1;
            map.serialize_entry(&move_number, mv)?;
        }

        map.end()
    }
}

struct SerializedGameEnding(GameEnding);

impl<'a> Serialize for SerializedGameEnding {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        let (result_str, reason_str) = match self.0 {
            GameEnding::Draw => ("1/2-1/2", "draw"),
            GameEnding::Stalemate => ("1/2-1/2", "stalemate"),
            GameEnding::WhiteWinsCheckmate => ("1-0", "checkmate"),
            GameEnding::BlackWinsCheckmate => ("0-1", "checkmate"),
            GameEnding::WhiteResigned => ("0-1", "resignation"),
            GameEnding::BlackResigned => ("1-0", "resignation"),
        };
        map.serialize_entry("result", result_str)?;
        map.serialize_entry("reason", reason_str)?;

        map.end()
    }
}

impl Serialize for DetailedGame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;

        map.serialize_entry("tags", &self.tags)?;

        map.serialize_entry("ending", &SerializedGameEnding(self.ending))?;
        map.serialize_entry("moves", &SerializedMoveList(&self.moves))?;

        map.end()
    }
}

fn write_yaml(yaml_path: &PathBuf, game: &DetailedGame) -> Result<()> {
    let output = std::fs::File::create(yaml_path).context(format!(
        "Could not open file {} for writing",
        yaml_path.to_string_lossy()
    ))?;

    serde_yaml::to_writer(output, game).context(format!(
        "Error writing YAML data to {}",
        yaml_path.to_string_lossy()
    ))
}

fn write_pgn(pgn: &PGN, path: &PathBuf) -> Result<()> {
    let detailed_game =
        pgn_to_long_algebraic(&pgn).context("Cannot convert to long algebraic form")?;
    write_yaml(&path, &detailed_game)?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let pgn_path = PathBuf::from(cli.file);

    let pgns = parse_pgn_file(&pgn_path).context(format!(
        "Unable to parse PGN file {}",
        pgn_path.to_string_lossy()
    ))?;

    let yaml_path = cli.output.map_or(
        {
            let mut path = pgn_path;
            path.set_extension("yaml");
            println!("Writing YAML file to {}", path.to_string_lossy());
            path
        },
        PathBuf::from,
    );

    match pgns.len() {
        0 => bail!("No games found in PGN file"),
        1 => write_pgn(&pgns[0], &yaml_path)?,
        _ => {
            for (index, pgn) in pgns.iter().enumerate() {
                let mut path = yaml_path.clone();
                path.set_file_name(format!(
                    "{}.{}.{}",
                    path.file_stem().unwrap().to_string_lossy(),
                    index,
                    path.extension().unwrap().to_string_lossy()
                ));
                write_pgn(&pgn, &path)?;
            }
        }
    }

    Ok(())
}
