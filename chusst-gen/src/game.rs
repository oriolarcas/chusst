mod play;
mod tree;
mod zobrist;

use std::fmt;

use anyhow::Result;
use serde::ser::SerializeMap;
use serde::Serialize;

use crate::board::{Board, ModifiableBoard, Piece, PieceType, Player, Position, SimpleBoard};
use crate::{mv, pos};
pub use tree::{AddNode, GameTree, TreeNode};
pub use zobrist::ZobristHash as GameHash;
pub use zobrist::ZobristHashBuilder as GameHashBuilder;

// Exports
pub use play::ModifiableGame;

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
            .and_then(PromotionPieces::try_from_char);

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

pub trait CastlingRights {
    fn can_castle_kingside(&self, player: Player) -> bool;
    fn can_castle_queenside(&self, player: Player) -> bool;
    fn disable_castle_kingside(&mut self, player: Player);
    fn disable_castle_queenside(&mut self, player: Player);
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
}

impl CastlingRights for GameInfo {
    fn can_castle_kingside(&self, player: Player) -> bool {
        match player {
            Player::White => self.white_kingside_castling_allowed,
            Player::Black => self.black_kingside_castling_allowed,
        }
    }

    fn can_castle_queenside(&self, player: Player) -> bool {
        match player {
            Player::White => self.white_queenside_castling_allowed,
            Player::Black => self.black_queenside_castling_allowed,
        }
    }

    fn disable_castle_kingside(&mut self, player: Player) {
        match player {
            Player::White => self.white_kingside_castling_allowed = false,
            Player::Black => self.black_kingside_castling_allowed = false,
        }
    }

    fn disable_castle_queenside(&mut self, player: Player) {
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

#[derive(Clone, Debug, PartialEq)]
pub struct GameMobilityData {
    player: Player,
    last_move: Option<MoveInfo>,
    info: GameInfo,
    hash: Option<GameHash>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameState<B: Board> {
    board: B,
    data: GameMobilityData,
}

impl<B: Board> Serialize for GameState<B> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        map.serialize_entry("board", &self.board)?;
        map.serialize_entry("player", &self.data.player)?;

        map.end()
    }
}

impl<B: Board> From<B> for GameState<B> {
    fn from(value: B) -> Self {
        GameState {
            board: value,
            data: GameMobilityData {
                player: Player::White,
                last_move: None,
                info: GameInfo::new(),
                hash: None,
            },
        }
    }
}

impl<B: Board> GameState<B> {
    pub const fn new() -> GameState<B> {
        GameState {
            board: B::NEW_BOARD,
            data: GameMobilityData {
                player: Player::White,
                last_move: None,
                info: GameInfo::new(),
                hash: None,
            },
        }
    }

    pub fn clone_unhashed(&self) -> Self {
        let mut new_game = self.clone();
        new_game.data.hash = None;
        new_game
    }

    pub fn data(&self) -> &GameMobilityData {
        &self.data
    }

    pub fn set_data(&mut self, data: &GameMobilityData) {
        self.data = data.clone();
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
            data: GameMobilityData {
                player,
                last_move,
                info,
                hash: None,
            },
        })
    }

    pub fn to_fen(&self) -> String {
        let mut fen = self.board.to_fen();
        let player = match self.data.player {
            Player::White => "w",
            Player::Black => "b",
        };
        let mut castling = format!(
            "{}{}{}{}",
            if self.data.info.can_castle_kingside(Player::White) {
                "K"
            } else {
                ""
            },
            if self.data.info.can_castle_queenside(Player::White) {
                "Q"
            } else {
                ""
            },
            if self.data.info.can_castle_kingside(Player::Black) {
                "k"
            } else {
                ""
            },
            if self.data.info.can_castle_queenside(Player::Black) {
                "q"
            } else {
                ""
            }
        );

        if castling.is_empty() {
            castling = "-".to_string();
        }

        let en_passant = match self.data.last_move {
            Some(MoveInfo {
                mv: Move { source: _, target },
                info: MoveExtraInfo::EnPassant,
            }) => {
                let rank = match self.data.player {
                    Player::White => target.rank + 1,
                    Player::Black => target.rank - 1,
                };
                format!("{}", pos!(rank, target.file))
            }
            _ => "-".to_string(),
        };

        fen.push_str(&format!(" {} {} {} 0 1", player, castling, en_passant));

        fen
    }

    pub fn hash(&mut self) -> GameHash {
        if let Some(hash) = self.data.hash {
            return hash;
        }

        let mut hash = GameHash::from(&self.board);

        if self.data.player == Player::Black {
            hash.switch_turn();
        }

        for player in [Player::White, Player::Black] {
            if !self.data.info.can_castle_kingside(player) {
                hash.switch_kingside_castling(player);
            }
            if !self.data.info.can_castle_queenside(player) {
                hash.switch_queenside_castling(player);
            }
        }

        if let Some(MoveInfo {
            mv,
            info: MoveExtraInfo::Passed,
        }) = self.data.last_move
        {
            hash.switch_en_passant_file(mv.target.file);
        }

        self.data.hash = Some(hash);

        hash
    }
}

impl<B: Board> ModifiableBoard<Position, Option<Piece>> for GameState<B> {
    fn at(&self, pos: &Position) -> Option<Piece> {
        self.board.at(pos)
    }

    fn update(&mut self, pos: &Position, value: Option<Piece>) {
        if let Some(hash) = self.data.hash.as_mut() {
            hash.update_piece(pos, self.board.at(pos), value);
        }
        self.board.update(pos, value);
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        if let Some(hash) = self.data.hash.as_mut() {
            if let Some(captured_piece) = self.board.at(target) {
                hash.update_piece(target, Some(captured_piece), None);
            }
            if let Some(moved_piece) = self.board.at(source) {
                hash.move_piece(source, target, moved_piece);
            }
        }
        self.board.move_piece(source, target);
    }
}

impl<B: Board> CastlingRights for GameState<B> {
    fn can_castle_kingside(&self, player: Player) -> bool {
        self.data.info.can_castle_kingside(player)
    }

    fn can_castle_queenside(&self, player: Player) -> bool {
        self.data.info.can_castle_queenside(player)
    }

    fn disable_castle_kingside(&mut self, player: Player) {
        if !self.data.info.can_castle_kingside(player) {
            return;
        }
        self.data.info.disable_castle_kingside(player);
        if let Some(hash) = self.data.hash.as_mut() {
            hash.switch_kingside_castling(player);
        }
    }

    fn disable_castle_queenside(&mut self, player: Player) {
        if !self.data.info.can_castle_queenside(player) {
            return;
        }
        self.data.info.disable_castle_queenside(player);
        if let Some(hash) = self.data.hash.as_mut() {
            hash.switch_queenside_castling(player);
        }
    }
}

// Board representations

#[cfg(feature = "bitboards")]
pub type BitboardGame = GameState<crate::board::Bitboards>;
#[cfg(feature = "compact-board")]
pub type CompactGame = GameState<crate::board::CompactBoard>;

pub type SimpleGame = GameState<SimpleBoard>;
