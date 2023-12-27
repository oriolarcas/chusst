use crate::board::{Board, PieceType, Player, Position, INITIAL_BOARD};
use crate::pos;

use serde::Serialize;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Move {
    pub source: Position,
    pub target: Position,
}

#[macro_export]
macro_rules! mv {
    ($src:expr, $tgt:expr) => {
        Move {
            source: $src,
            target: $tgt,
        }
    };
    ($src:ident => $tgt:ident) => {
        Move {
            source: pos!($src),
            target: pos!($tgt),
        }
    };
}

impl Move {
    pub fn try_from_long_algebraic_str(mv_str: &str) -> Option<Move> {
        if mv_str.len() != 4 {
            return None;
        }
        let source = Position::try_from_str(&mv_str[0..2]);
        let target = Position::try_from_str(&mv_str[2..4]);

        match (source, target) {
            (Some(src_mv), Some(tgt_mv)) => Some(mv!(src_mv, tgt_mv)),
            _ => None,
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.source, self.target)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum MoveExtraInfo {
    Other,
    Promotion(PieceType),
    Passed,
    EnPassant,
    CastleKingside,
    CastleQueenside,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct MoveInfo {
    pub mv: Move,
    pub info: MoveExtraInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct GameInfo {
    white_kingside_castling_allowed: bool,
    white_queenside_castling_allowed: bool,
    black_kingside_castling_allowed: bool,
    black_queenside_castling_allowed: bool,
}

impl GameInfo {
    pub const fn new() -> GameInfo {
        Self {
            white_kingside_castling_allowed: true,
            white_queenside_castling_allowed: true,
            black_kingside_castling_allowed: true,
            black_queenside_castling_allowed: true,
        }
    }

    pub fn can_castle_kingside(&self, player: &Player) -> bool {
        match player {
            Player::White => self.white_kingside_castling_allowed,
            Player::Black => self.black_kingside_castling_allowed,
        }
    }

    pub fn can_castle_queenside(&self, player: &Player) -> bool {
        match player {
            Player::White => self.white_queenside_castling_allowed,
            Player::Black => self.black_queenside_castling_allowed,
        }
    }

    pub fn disable_castle_kingside(&mut self, player: &Player) {
        match player {
            Player::White => self.white_kingside_castling_allowed = false,
            Player::Black => self.black_kingside_castling_allowed = false,
        }
    }

    pub fn disable_castle_queenside(&mut self, player: &Player) {
        match player {
            Player::White => self.white_queenside_castling_allowed = false,
            Player::Black => self.black_queenside_castling_allowed = false,
        }
    }
}

impl fmt::Display for GameInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{white: {{kingside: {}, queenside: {}}}, black: {{kingside: {}, queenside: {}}}",
            self.white_kingside_castling_allowed,
            self.white_queenside_castling_allowed,
            self.black_kingside_castling_allowed,
            self.black_queenside_castling_allowed
        )
    }
}

impl Default for GameInfo {
    fn default() -> Self {
        GameInfo::new()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Game {
    pub board: Board,
    pub player: Player,
    pub last_move: Option<MoveInfo>,
    pub info: GameInfo,
}

impl Game {
    pub const fn new() -> Game {
        Game {
            board: INITIAL_BOARD,
            player: Player::White,
            last_move: None,
            info: GameInfo::new(),
        }
    }

    pub fn try_from_fen(fen: &[&str]) -> Option<Game> {
        // rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
        // ^                                           ^ ^    ^ ^ ^
        // |                                           | |    | | ` Fullmove number
        // |                                           | |    | ` Halfmove clock
        // |                                           | |    ` En passant target square
        // |                                           | ` Castling availability
        // |                                           ` Active color
        // ` Pieces

        if fen.len() != 6 {
            return None;
        }

        let [pieces, player_str, castling, en_passant, _halfmove, _fullmove] = fen else { return None; };

        let board = Board::try_from_fen(pieces)?;

        let player = match *player_str {
            "w" => Player::White,
            "b" => Player::Black,
            _ => return None,
        };

        let last_move = if *en_passant != "-" {
            let en_passant_pos = Position::try_from_str(en_passant)?;
            // Player who made the en passant in the previous move
            let passed_pawn_player = match player {
                Player::White => Player::Black,
                Player::Black => Player::White,
            };
            let (source_rank, passed_rank, target_rank) = match passed_pawn_player {
                Player::White => (1, 2, 3),
                Player::Black => (6, 5, 4),
            };

            if en_passant_pos.rank != passed_rank {
                return None;
            }

            Some(MoveInfo {
                mv: mv!(
                    pos!(source_rank, en_passant_pos.file),
                    pos!(target_rank, en_passant_pos.file)
                ),
                info: MoveExtraInfo::EnPassant,
            })
        } else {
            None
        };

        let info = GameInfo {
            white_kingside_castling_allowed: castling.contains('K'),
            white_queenside_castling_allowed: castling.contains('Q'),
            black_kingside_castling_allowed: castling.contains('k'),
            black_queenside_castling_allowed: castling.contains('q'),
        };

        Some(Game {
            board,
            player,
            last_move,
            info,
        })
    }
}
