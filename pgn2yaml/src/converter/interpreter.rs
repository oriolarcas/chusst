use crate::reader::{Tag, Pgn};
use anyhow::{bail, Result};
use chusst_gen::{
    board::{Piece, PieceType, ModifiableBoard},
    game::{SimpleGame, MoveAction, ModifiableGame}, eval::Game,
};

#[derive(PartialEq)]
pub enum PromotionPieces {
    Knight,
    Bishop,
    Rook,
    Queen,
}

#[derive(PartialEq)]
pub enum MoveType {
    Normal,
    Capture,
    PassingPawn,
    EnPassant,
    Promotion(PromotionPieces),
    PromotionWithCapture(PromotionPieces),
    KingsideCastling,
    QueensideCastling,
}

pub enum CheckType {
    Check,
    Checkmate,
    #[allow(dead_code)]
    Stalemate,
}

pub struct DetailedMoveInfo {
    pub mv: MoveAction,
    pub short: String,
    pub long: String,
    pub move_type: MoveType,
    pub check_type: Option<CheckType>,
}

pub struct DetailedMove {
    pub white: DetailedMoveInfo,
    pub black: Option<DetailedMoveInfo>,
}

#[derive(Default, Clone, Copy)]
pub enum GameEnding {
    #[default]
    Draw,
    Stalemate,
    WhiteWinsCheckmate,
    BlackWinsCheckmate,
    WhiteResigned,
    BlackResigned,
}

#[derive(Default)]
pub struct DetailedGame {
    pub tags: Vec<Tag>,
    pub moves: Vec<DetailedMove>,
    pub ending: GameEnding,
}

fn long(mv: &chusst_gen::game::Move, capture: bool) -> String {
    format!(
        "{}{}{}",
        mv.source,
        if capture { "x" } else { "-" },
        mv.target
    )
}

fn find_move_by_name(game: &SimpleGame, move_str: &str) -> Result<DetailedMoveInfo> {
    let possible_moves = game.get_all_possible_moves();
    for move_action in &possible_moves {
        let mv = &move_action.mv;
        let mv_name = game.move_name(move_action).unwrap();

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
                        mv: *move_action,
                        short: move_str.to_string(),
                        long: format!(
                            "{}{}",
                            long(mv, is_capture),
                            promoted_piece_char.to_lowercase()
                        ),
                        move_type,
                        check_type,
                    });
                }
            }

            let Some(Piece { piece, player: _ }) = game.at(&mv.source) else {
                bail!("Source square is empty");
            };
            let target_empty = game.at(&mv.target).is_none();
            let mv_rank_distance = mv.source.rank.abs_diff(mv.target.rank);
            let mv_file_distance = mv.source.file.abs_diff(mv.target.file);

            return Ok(DetailedMoveInfo {
                mv: *move_action,
                short: move_str.to_string(),
                long: long(mv, is_capture),
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
        game.player(),
        game.board(),
        &possible_moves
            .iter()
            .filter_map(|mv| game.move_name(mv))
            .collect::<Vec<String>>()
            .join(", ")
    );
}

fn is_stalemate(game: &SimpleGame) -> bool {
    game.get_all_possible_moves().is_empty()
}

pub fn pgn_to_long_algebraic(pgn: &Pgn) -> Result<DetailedGame> {
    let mut game = SimpleGame::new();
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

        game.do_move(&detailed_white_mv.mv);

        if let Some(black_short_str) = mv.black.as_deref() {
            let detailed_black_mv = find_move_by_name(&game, black_short_str)?;
            game.do_move(&detailed_black_mv.mv);

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
