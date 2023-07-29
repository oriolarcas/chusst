use serde::Serialize;
use std::fmt;

#[derive(Copy, Clone, PartialEq, Serialize)]
pub enum PieceType {
    Pawn,   // p
    Knight, // n
    Bishop, // b
    Rook,   // r
    Queen,  // q
    King,   // k
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                PieceType::Pawn => "pawn",
                PieceType::Knight => "knight",
                PieceType::Bishop => "bishop",
                PieceType::Rook => "rook",
                PieceType::Queen => "queen",
                PieceType::King => "king",
            }
        )
    }
}

#[derive(Copy, Clone, Serialize, PartialEq)]
pub enum Player {
    White,
    Black,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Player::White => "white",
                Player::Black => "black",
            }
        )
    }
}

#[derive(Copy, Clone, Serialize)]
pub struct Piece {
    pub piece: PieceType,
    pub player: Player,
}

macro_rules! p {
    (pw) => {
        Some(Piece {
            piece: PieceType::Pawn,
            player: Player::White,
        })
    };
    (nw) => {
        Some(Piece {
            piece: PieceType::Knight,
            player: Player::White,
        })
    };
    (bw) => {
        Some(Piece {
            piece: PieceType::Bishop,
            player: Player::White,
        })
    };
    (rw) => {
        Some(Piece {
            piece: PieceType::Rook,
            player: Player::White,
        })
    };
    (qw) => {
        Some(Piece {
            piece: PieceType::Queen,
            player: Player::White,
        })
    };
    (kw) => {
        Some(Piece {
            piece: PieceType::King,
            player: Player::White,
        })
    };
    (pb) => {
        Some(Piece {
            piece: PieceType::Pawn,
            player: Player::Black,
        })
    };
    (nb) => {
        Some(Piece {
            piece: PieceType::Knight,
            player: Player::Black,
        })
    };
    (bb) => {
        Some(Piece {
            piece: PieceType::Bishop,
            player: Player::Black,
        })
    };
    (rb) => {
        Some(Piece {
            piece: PieceType::Rook,
            player: Player::Black,
        })
    };
    (qb) => {
        Some(Piece {
            piece: PieceType::Queen,
            player: Player::Black,
        })
    };
    (kb) => {
        Some(Piece {
            piece: PieceType::King,
            player: Player::Black,
        })
    };
    () => {
        Option::<Piece>::None
    };
}

pub type Rank<T> = [T; 8];
pub type Square = Option<Piece>;
pub type Row = Rank<Square>;
pub type Rows = Rank<Row>;

#[derive(Copy, Clone, Serialize)]
pub struct Board {
    // rows[x][y], where x = 0..7 = rows 1..8, and y = 0..7 = columns a..h
    // for instance, e4 is Board.rows[2][4]
    pub rows: Rows,
}

#[derive(Copy, Clone, PartialEq, Serialize)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let row = self.row + 1;
        let col = ["a", "b", "c", "d", "e", "f", "g", "h"][self.col];
        write!(f, "{}{}", col, row)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Move {
    pub source: Position,
    pub target: Position,
}

#[derive(Copy, Clone, Serialize)]
pub struct Game {
    pub board: Board,
    pub player: Player,
    pub turn: u32,
}
