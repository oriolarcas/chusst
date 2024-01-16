use crate::board::{
    Board, CompactBoard, ModifiableBoard, Piece, PieceType, Player, Position, SimpleBoard,
};
use crate::{mv, pos};

use serde::Serialize;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Move {
    pub source: Position,
    pub target: Position,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub enum PromotionPieces {
    Knight,
    Bishop,
    Rook,
    Queen,
}

impl PromotionPieces {
    pub fn try_from_str(value: String) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "knight" => Some(PromotionPieces::Knight),
            "bishop" => Some(PromotionPieces::Bishop),
            "rook" => Some(PromotionPieces::Rook),
            "queen" => Some(PromotionPieces::Queen),
            _ => None,
        }
    }

    pub fn try_from_char(value: char) -> Option<Self> {
        match value.to_ascii_lowercase() {
            'k' => Some(PromotionPieces::Knight),
            'b' => Some(PromotionPieces::Bishop),
            'r' => Some(PromotionPieces::Rook),
            'q' => Some(PromotionPieces::Queen),
            _ => None,
        }
    }
}

impl From<PromotionPieces> for PieceType {
    fn from(value: PromotionPieces) -> Self {
        match value {
            PromotionPieces::Knight => PieceType::Knight,
            PromotionPieces::Bishop => PieceType::Bishop,
            PromotionPieces::Rook => PieceType::Rook,
            PromotionPieces::Queen => PieceType::Queen,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum MoveActionType {
    Normal,
    Promotion(PromotionPieces),
}

#[derive(Copy, Clone, PartialEq)]
pub struct MoveAction {
    pub mv: Move,
    pub move_type: MoveActionType,
}

impl MoveAction {
    pub fn try_from_long_algebraic_str(mv_str: &str) -> Option<MoveAction> {
        if mv_str.len() < 4 || mv_str.len() > 5 {
            return None;
        }
        let source = Position::try_from_str(&mv_str[0..2]);
        let target = Position::try_from_str(&mv_str[2..4]);
        let promotion = mv_str
            .chars()
            .nth(4)
            .map(PromotionPieces::try_from_char)
            .flatten();

        match (source, target, promotion) {
            (Some(src_pos), Some(tgt_pos), None) => Some(MoveAction {
                mv: mv!(src_pos, tgt_pos),
                move_type: MoveActionType::Normal,
            }),
            (Some(src_pos), Some(tgt_pos), Some(promotion_piece)) => Some(MoveAction {
                mv: mv!(src_pos, tgt_pos),
                move_type: MoveActionType::Promotion(promotion_piece),
            }),
            _ => None,
        }
    }
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

#[macro_export]
macro_rules! mva {
    ($src:expr, $tgt:expr) => {
        MoveAction {
            mv: Move {
                source: $src,
                target: $tgt,
            },
            move_type: MoveActionType::Normal,
        }
    };
    ($src:expr, $tgt:expr, $promoted:expr) => {
        MoveAction {
            mv: Move {
                source: $src,
                target: $tgt,
            },
            move_type: MoveActionType::Promotion($promoted),
        }
    };
    ($src:ident => $tgt:ident) => {
        MoveAction {
            mv: Move {
                source: pos!($src),
                target: pos!($tgt),
            },
            move_type: MoveActionType::Normal,
        }
    };
    ($src:ident => $tgt:ident, $promoted:expr) => {
        MoveAction {
            mv: Move {
                source: pos!($src),
                target: pos!($tgt),
            },
            move_type: MoveActionType::Promotion($promoted),
        }
    };
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.source, self.target)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum MoveExtraInfo {
    Other,
    Promotion(PromotionPieces),
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GameState<B: Board> {
    pub(crate) board: B,
    pub(crate) player: Player,
    pub(crate) last_move: Option<MoveInfo>,
    pub(crate) info: GameInfo,
}

impl<B: Board> From<B> for GameState<B> {
    fn from(value: B) -> Self {
        GameState {
            board: value,
            player: Player::White,
            last_move: None,
            info: GameInfo::new(),
        }
    }
}

impl<B: Board> GameState<B> {
    pub const fn new() -> GameState<B> {
        GameState {
            board: B::NEW_BOARD,
            player: Player::White,
            last_move: None,
            info: GameInfo::new(),
        }
    }

    pub fn player(&self) -> Player {
        self.player
    }

    pub fn board(&self) -> &B {
        &self.board
    }

    pub fn try_from_fen(fen: &[&str]) -> Option<Self> {
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

        let [pieces, player_str, castling, en_passant, _halfmove, _fullmove] = fen else {
            return None;
        };

        let board = B::try_from_fen(pieces)?;

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

        Some(GameState {
            board,
            player,
            last_move,
            info,
        })
    }
}

impl<B: Board> ModifiableBoard<Position, Option<Piece>> for GameState<B> {
    fn at(&self, pos: &Position) -> Option<Piece> {
        self.board.at(pos)
    }

    fn update(&mut self, pos: &Position, value: Option<Piece>) {
        self.board.update(pos, value)
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        self.board.move_piece(source, target)
    }
}

// Board representations

#[cfg(feature = "bitboards")]
pub type BitboardGame = GameState<crate::board::Bitboards>;

pub type SimpleGame = GameState<SimpleBoard>;
pub type CompactGame = GameState<CompactBoard>;
