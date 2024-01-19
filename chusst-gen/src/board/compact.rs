use std::fmt;

use serde::Serialize;

use super::{
    format_board, serialize_board, Board, IterableBoard, ModifiableBoard, Piece, PieceType, Player,
    Position, Ranks,
};
use crate::p;

// Binary representation of a square:
// 0b000vPppp where:
// v = 1 if the square is valid, 0 if it is not
// P = 0 if the piece is white, 1 if it is black
// pppp = piece type, 0..5 = pawn..king
#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct Square(u8);

#[allow(dead_code)]
const SIZE_ASSERTION: [u8; 64] = [0; std::mem::size_of::<Ranks<Square>>()];

impl Square {
    const fn from(value: Option<Piece>) -> Self {
        match value {
            Some(piece) => {
                let mut square_byte = 0b0010_0000u8;
                square_byte |= match piece.player {
                    Player::White => 0b0000_0000,
                    Player::Black => 0b0001_0000,
                };
                square_byte |= match piece.piece {
                    PieceType::Pawn => 0b0000_0000,
                    PieceType::Knight => 0b0000_0001,
                    PieceType::Bishop => 0b0000_0010,
                    PieceType::Rook => 0b0000_0011,
                    PieceType::Queen => 0b0000_0100,
                    PieceType::King => 0b0000_0101,
                };
                Square(square_byte)
            }
            None => Square(0),
        }
    }

    const fn into(self) -> Option<Piece> {
        if self.0 & 0b0010_0000 == 0 {
            return None;
        }

        let player = match self.0 & 0b0001_0000 != 0 {
            true => Player::Black,
            false => Player::White,
        };

        let piece = match self.0 & 0b0000_1111 {
            0 => PieceType::Pawn,
            1 => PieceType::Knight,
            2 => PieceType::Bishop,
            3 => PieceType::Rook,
            4 => PieceType::Queen,
            5 => PieceType::King,
            _ => unreachable!(),
        };

        Some(Piece { piece, player })
    }
}

impl From<Option<Piece>> for Square {
    fn from(value: Option<Piece>) -> Self {
        Square::from(value)
    }
}

impl Into<Option<Piece>> for Square {
    fn into(self) -> Option<Piece> {
        Square::into(self)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompactBoard {
    ranks: Ranks<Square>,
}

macro_rules! cp {
    ($piece:ident) => {
        Square::from(p!($piece))
    };

    () => {
        Self::EMPTY_SQUARE
    };
}

impl CompactBoard {
    const EMPTY_SQUARE: Square = Square(0b0000_0000);

    #[rustfmt::skip]
    pub const fn new() -> Self {
        Self {
            ranks: [
                [cp!(rw), cp!(nw), cp!(bw), cp!(qw), cp!(kw), cp!(bw), cp!(nw), cp!(rw)],
                [cp!(pw), cp!(pw), cp!(pw), cp!(pw), cp!(pw), cp!(pw), cp!(pw), cp!(pw)],
                [cp!(); 8],
                [cp!(); 8],
                [cp!(); 8],
                [cp!(); 8],
                [cp!(pb), cp!(pb), cp!(pb), cp!(pb), cp!(pb), cp!(pb), cp!(pb), cp!(pb)],
                [cp!(rb), cp!(nb), cp!(bb), cp!(qb), cp!(kb), cp!(bb), cp!(nb), cp!(rb)],
            ],
        }
    }
}

impl ModifiableBoard<Position, Option<Piece>> for CompactBoard {
    fn at(&self, pos: &Position) -> Option<Piece> {
        self.ranks[pos.rank][pos.file].into()
    }

    fn update(&mut self, pos: &Position, value: Option<Piece>) {
        self.ranks[pos.rank][pos.file] = value.into();
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        self.ranks[target.rank][target.file] = self.ranks[source.rank][source.file];
        self.ranks[source.rank][source.file] = Self::EMPTY_SQUARE;
    }
}

impl IterableBoard for CompactBoard {}

impl Board for CompactBoard {
    const NEW_BOARD: Self = CompactBoard::new();
}

impl Serialize for CompactBoard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_board(self, serializer)
    }
}

impl fmt::Display for CompactBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format_board(self, f)
    }
}
