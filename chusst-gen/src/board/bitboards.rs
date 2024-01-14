mod attack;
mod in_between;

use serde::Serialize;

use super::{
    format_board, serialize_board, Board, ModifiableBoard, Piece, PieceType, Player, Position,
};
use std::fmt;

type Bitboard = u64;

fn rank_and_file_to_bitboard_index(rank: usize, file: usize) -> usize {
    rank * 8 + file
}

fn position_to_bitboard_index(position: &Position) -> usize {
    rank_and_file_to_bitboard_index(position.rank, position.file)
}

fn bitboard_from_rank_and_file(rank: usize, file: usize) -> Bitboard {
    1 << rank_and_file_to_bitboard_index(rank, file)
}

fn bitboard_from_position(position: &Position) -> Bitboard {
    bitboard_from_rank_and_file(position.rank, position.file)
}

fn check_bitboard(bitboard: Bitboard, position: &Position) -> bool {
    bitboard & bitboard_from_position(position) != 0
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerBitboards {
    player: Player,
    pawns: Bitboard,
    knights: Bitboard,
    bishops: Bitboard,
    rooks: Bitboard,
    queens: Bitboard,
    kings: Bitboard,
    combined: Bitboard,
}

pub struct BitboardIter {
    bitboard: Bitboard,
}

impl Iterator for BitboardIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bitboard == 0 {
            return None;
        }

        let index = self.bitboard.trailing_zeros() as usize;
        self.bitboard &= !(1 << index);

        let rank = index / 8;
        let file = index % 8;

        Some(Position { rank, file })
    }
}

impl PlayerBitboards {
    fn apply<F>(&mut self, mut f: F)
    where
        F: FnMut(Bitboard) -> Bitboard,
    {
        self.pawns = f(self.pawns);
        self.knights = f(self.knights);
        self.bishops = f(self.bishops);
        self.rooks = f(self.rooks);
        self.queens = f(self.queens);
        self.kings = f(self.kings);
    }

    pub const fn default(player: Player) -> PlayerBitboards {
        PlayerBitboards {
            player,
            pawns: 0,
            knights: 0,
            bishops: 0,
            rooks: 0,
            queens: 0,
            kings: 0,
            combined: 0,
        }
    }

    #[rustfmt::skip]
    pub const fn new(player: Player) -> PlayerBitboards {
        const WHITE_PAWNS: Bitboard   = 0b00000000_00000000_00000000_00000000_00000000_00000000_11111111_00000000;
        const WHITE_KNIGHTS: Bitboard = 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_01000010;
        const WHITE_BISHOPS: Bitboard = 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00100100;
        const WHITE_ROOKS: Bitboard   = 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_10000001;
        const WHITE_QUEENS: Bitboard  = 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00001000;
        const WHITE_KINGS: Bitboard   = 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00010000;

        const BLACK_PAWNS: Bitboard   = 0b00000000_11111111_00000000_00000000_00000000_00000000_00000000_00000000;
        const BLACK_KNIGHTS: Bitboard = 0b01000010_00000000_00000000_00000000_00000000_00000000_00000000_00000000;
        const BLACK_BISHOPS: Bitboard = 0b00100100_00000000_00000000_00000000_00000000_00000000_00000000_00000000;
        const BLACK_ROOKS: Bitboard   = 0b10000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000;
        const BLACK_QUEENS: Bitboard  = 0b00001000_00000000_00000000_00000000_00000000_00000000_00000000_00000000;
        const BLACK_KINGS: Bitboard   = 0b00010000_00000000_00000000_00000000_00000000_00000000_00000000_00000000;

        PlayerBitboards {
            player,
            pawns:   match player { Player::White => WHITE_PAWNS,   Player::Black => BLACK_PAWNS   },
            knights: match player { Player::White => WHITE_KNIGHTS, Player::Black => BLACK_KNIGHTS },
            bishops: match player { Player::White => WHITE_BISHOPS, Player::Black => BLACK_BISHOPS },
            rooks:   match player { Player::White => WHITE_ROOKS,   Player::Black => BLACK_ROOKS   },
            queens:  match player { Player::White => WHITE_QUEENS,  Player::Black => BLACK_QUEENS  },
            kings:   match player { Player::White => WHITE_KINGS,   Player::Black => BLACK_KINGS   },
            combined: match player {
                Player::White => WHITE_PAWNS | WHITE_KNIGHTS | WHITE_BISHOPS | WHITE_ROOKS | WHITE_QUEENS | WHITE_KINGS,
                Player::Black => BLACK_PAWNS | BLACK_KNIGHTS | BLACK_BISHOPS | BLACK_ROOKS | BLACK_QUEENS | BLACK_KINGS
            },
        }
    }

    pub fn has_position(&self, position: &Position) -> bool {
        check_bitboard(self.combined(), position)
    }

    pub fn into_iter(bitboard: Bitboard) -> BitboardIter {
        BitboardIter { bitboard }
    }

    pub fn piece_iter(&self, piece: &PieceType) -> BitboardIter {
        BitboardIter {
            bitboard: self.by_piece(piece),
        }
    }

    pub fn combined(&self) -> Bitboard {
        self.pawns | self.knights | self.bishops | self.rooks | self.queens | self.kings
    }

    pub fn by_piece(&self, piece: &PieceType) -> Bitboard {
        match piece {
            PieceType::Pawn => self.pawns,
            PieceType::Knight => self.knights,
            PieceType::Bishop => self.bishops,
            PieceType::Rook => self.rooks,
            PieceType::Queen => self.queens,
            PieceType::King => self.kings,
        }
    }

    // Tables

    pub fn in_between(source: &Position, target: &Position) -> Bitboard {
        let source_index = position_to_bitboard_index(source);
        let target_index = position_to_bitboard_index(target);
        in_between::IN_BETWEEN_TABLE[source_index][target_index]
    }

    pub fn pawn_can_attack(&self, target_position: &Position) -> bool {
        let target_index = position_to_bitboard_index(target_position);
        match self.player {
            Player::White => self.pawns & attack::BLACK_ATTACKED_BY_PAWN_TABLE[target_index] != 0,
            Player::Black => self.pawns & attack::WHITE_ATTACKED_BY_PAWN_TABLE[target_index] != 0,
        }
    }

    pub fn knight_can_attack(&self, target_position: &Position) -> bool {
        let target_index = position_to_bitboard_index(target_position);
        self.knights & attack::ATTACKED_BY_KNIGHT_TABLE[target_index] != 0
    }

    pub fn king_can_attack(&self, target_position: &Position) -> bool {
        let target_index = position_to_bitboard_index(target_position);
        self.kings & attack::ATTACKED_BY_KING_TABLE[target_index] != 0
    }
}

impl ModifiableBoard<Position, Option<PieceType>> for PlayerBitboards {
    fn at(&self, index: &Position) -> Option<PieceType> {
        let position_mask = bitboard_from_position(&index);
        if self.pawns & position_mask != 0 {
            Some(PieceType::Pawn)
        } else if self.knights & position_mask != 0 {
            Some(PieceType::Knight)
        } else if self.bishops & position_mask != 0 {
            Some(PieceType::Bishop)
        } else if self.rooks & position_mask != 0 {
            Some(PieceType::Rook)
        } else if self.queens & position_mask != 0 {
            Some(PieceType::Queen)
        } else if self.kings & position_mask != 0 {
            Some(PieceType::King)
        } else {
            None
        }
    }

    fn update(&mut self, pos: &Position, value: Option<PieceType>) {
        let mask = bitboard_from_position(pos);
        match value {
            Some(piece) => {
                match piece {
                    PieceType::Pawn => self.pawns |= mask,
                    PieceType::Knight => self.knights |= mask,
                    PieceType::Bishop => self.bishops |= mask,
                    PieceType::Rook => self.rooks |= mask,
                    PieceType::Queen => self.queens |= mask,
                    PieceType::King => self.kings |= mask,
                }
                self.combined |= mask;
            }
            None => {
                let negate_mask = !mask;
                self.apply(|bitboard: Bitboard| bitboard & negate_mask);
                self.combined &= negate_mask;
            }
        }
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        let source_mask = bitboard_from_position(source);
        let set_mask = bitboard_from_position(target);
        let clear_mask = !set_mask;
        let move_bit = |bitboard: Bitboard| -> Bitboard {
            if bitboard & source_mask != 0 {
                bitboard | set_mask
            } else {
                bitboard & clear_mask
            }
        };
        self.apply(move_bit);
    }
}

impl fmt::Display for PlayerBitboards {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut board = Bitboards::default();

        for rank in 0..8 {
            for file in 0..8 {
                let pos = Position { rank, file };
                board.update(
                    &pos,
                    self.at(&pos).map(|piece| Piece {
                        piece,
                        player: self.player,
                    }),
                );
            }
        }

        write!(f, "{}", board)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bitboards {
    white: PlayerBitboards,
    black: PlayerBitboards,
}

impl Bitboards {
    pub const fn default() -> Self {
        Self {
            white: PlayerBitboards::default(Player::White),
            black: PlayerBitboards::default(Player::Black),
        }
    }

    pub const fn new() -> Self {
        Self {
            white: PlayerBitboards::new(Player::White),
            black: PlayerBitboards::new(Player::Black),
        }
    }

    pub fn by_player(&self, player: &crate::board::Player) -> &PlayerBitboards {
        match player {
            Player::White => &self.white,
            Player::Black => &self.black,
        }
    }

    pub fn player_at_position(&self, position: &Position) -> Option<Player> {
        if self.white.has_position(position) {
            Some(Player::White)
        } else if self.black.has_position(position) {
            Some(Player::Black)
        } else {
            None
        }
    }
}

impl ModifiableBoard<Position, Option<Piece>> for Bitboards {
    fn at(&self, pos: &Position) -> Option<Piece> {
        if let Some(piece) = self.white.at(pos) {
            Some(Piece {
                player: Player::White,
                piece,
            })
        } else if let Some(piece) = self.black.at(pos) {
            Some(Piece {
                player: Player::Black,
                piece,
            })
        } else {
            None
        }
    }

    fn update(&mut self, pos: &Position, value: Option<Piece>) {
        let value_player = value.map(|piece| piece.player);
        let value_piece = value.map(|piece| piece.piece);
        let player_at_position = self.player_at_position(pos);

        match value_player {
            Some(player) => match player {
                Player::White => {
                    if Some(Player::Black) == player_at_position {
                        self.black.update(pos, None);
                    }
                    self.white.update(pos, value_piece);
                }
                Player::Black => {
                    if Some(Player::White) == player_at_position {
                        self.white.update(pos, None);
                    }
                    self.black.update(pos, value_piece);
                }
            },
            None => {
                if Some(Player::White) == player_at_position {
                    self.white.update(pos, None);
                }
                if Some(Player::Black) == player_at_position {
                    self.black.update(pos, None);
                }
            }
        }
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        let source_square = self.at(source);
        self.update(target, source_square);
        self.update(source, None);
    }
}

impl Board for Bitboards {
    const NEW_BOARD: Self = Bitboards::new();
}

impl Default for Bitboards {
    fn default() -> Self {
        Self::default()
    }
}

impl Serialize for Bitboards {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_board(self, serializer)
    }
}

impl fmt::Display for Bitboards {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format_board(self, f)
    }
}
