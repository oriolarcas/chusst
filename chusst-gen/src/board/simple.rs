use core::fmt;

use serde::Serialize;

use super::{
    format_board, serialize_board, Board, ModifiableBoard, Piece, PieceType, Player, Position,
    Ranks,
};
use crate::p;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SimpleBoard {
    ranks: Ranks<Option<Piece>>,
}

impl SimpleBoard {
    pub const fn new() -> Self {
        Self {
            ranks: [
                [
                    p!(rw),
                    p!(nw),
                    p!(bw),
                    p!(qw),
                    p!(kw),
                    p!(bw),
                    p!(nw),
                    p!(rw),
                ],
                [
                    p!(pw),
                    p!(pw),
                    p!(pw),
                    p!(pw),
                    p!(pw),
                    p!(pw),
                    p!(pw),
                    p!(pw),
                ],
                [p!(); 8],
                [p!(); 8],
                [p!(); 8],
                [p!(); 8],
                [
                    p!(pb),
                    p!(pb),
                    p!(pb),
                    p!(pb),
                    p!(pb),
                    p!(pb),
                    p!(pb),
                    p!(pb),
                ],
                [
                    p!(rb),
                    p!(nb),
                    p!(bb),
                    p!(qb),
                    p!(kb),
                    p!(bb),
                    p!(nb),
                    p!(rb),
                ],
            ],
        }
    }
}

impl ModifiableBoard<Position, Option<Piece>> for SimpleBoard {
    fn at(&self, pos: &Position) -> Option<Piece> {
        self.ranks[pos.rank][pos.file]
    }

    fn update(&mut self, pos: &Position, value: Option<Piece>) {
        self.ranks[pos.rank][pos.file] = value;
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        self.ranks[target.rank][target.file] = self.ranks[source.rank][source.file];
        self.ranks[source.rank][source.file] = None;
    }
}

impl Board for SimpleBoard {
    const NEW_BOARD: Self = SimpleBoard::new();
}

impl Serialize for SimpleBoard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_board(self, serializer)
    }
}

impl fmt::Display for SimpleBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format_board(self, f)
    }
}
