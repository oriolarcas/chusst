use serde::Serialize;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
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

#[derive(Copy, Clone, Debug, Serialize, PartialEq)]
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

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
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

pub type Square = Option<Piece>;

pub type Rank<T> = [T; 8];
pub type Row<T> = Rank<T>;
pub type Rows<T> = Rank<Row<T>>;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Board {
    // rows[x][y], where x = 0..7 = rows 1..8, and y = 0..7 = columns a..h
    // for instance, e4 is Board.rows[2][4]
    pub rows: Rows<Square>,
}

impl Board {
    pub fn square(&self, pos: Position) -> &Square {
        &self.rows[pos.row][pos.col]
    }
}

fn get_unicode_piece(piece: PieceType, player: Player) -> &'static str {
    match (player, piece) {
        (Player::White, PieceType::Pawn) => "♙",
        (Player::White, PieceType::Knight) => "♘",
        (Player::White, PieceType::Bishop) => "♗",
        (Player::White, PieceType::Rook) => "♖",
        (Player::White, PieceType::Queen) => "♕",
        (Player::White, PieceType::King) => "♔",
        (Player::Black, PieceType::Pawn) => "♟︎",
        (Player::Black, PieceType::Knight) => "♞",
        (Player::Black, PieceType::Bishop) => "♝",
        (Player::Black, PieceType::Rook) => "♜",
        (Player::Black, PieceType::Queen) => "♛",
        (Player::Black, PieceType::King) => "♚",
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rows: Vec<String> = Default::default();

        rows.push("   a  b  c  d  e  f  g  h ".to_owned());
        for (rank, row) in self.rows.iter().rev().enumerate() {
            let mut row_str = String::from(format!("{} ", 8 - rank));
            for square in row {
                let piece = match square {
                    Some(square_value) => get_unicode_piece(square_value.piece, square_value.player),
                    None => " ",
                };
                row_str += format!("[{}]", piece).as_str();
            }
            rows.push(row_str);
        }
        write!(f, "{}", rows.join("\n"))
    }
}

macro_rules! initial_board {
    () => {
        Board {
            // Initial board
            // Note that white pieces are at the top, because arrays are defined top-down, while chess rows go bottom-up
            rows: [
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

macro_rules! pos {
    (a1) => {
        Position { row: 0, col: 0 }
    };
    (b1) => {
        Position { row: 0, col: 1 }
    };
    (c1) => {
        Position { row: 0, col: 2 }
    };
    (d1) => {
        Position { row: 0, col: 3 }
    };
    (e1) => {
        Position { row: 0, col: 4 }
    };
    (f1) => {
        Position { row: 0, col: 5 }
    };
    (g1) => {
        Position { row: 0, col: 6 }
    };
    (h1) => {
        Position { row: 0, col: 7 }
    };
    (a2) => {
        Position { row: 1, col: 0 }
    };
    (b2) => {
        Position { row: 1, col: 1 }
    };
    (c2) => {
        Position { row: 1, col: 2 }
    };
    (d2) => {
        Position { row: 1, col: 3 }
    };
    (e2) => {
        Position { row: 1, col: 4 }
    };
    (f2) => {
        Position { row: 1, col: 5 }
    };
    (g2) => {
        Position { row: 1, col: 6 }
    };
    (h2) => {
        Position { row: 1, col: 7 }
    };
    (a3) => {
        Position { row: 2, col: 0 }
    };
    (b3) => {
        Position { row: 2, col: 1 }
    };
    (c3) => {
        Position { row: 2, col: 2 }
    };
    (d3) => {
        Position { row: 2, col: 3 }
    };
    (e3) => {
        Position { row: 2, col: 4 }
    };
    (f3) => {
        Position { row: 2, col: 5 }
    };
    (g3) => {
        Position { row: 2, col: 6 }
    };
    (h3) => {
        Position { row: 2, col: 7 }
    };
    (a4) => {
        Position { row: 3, col: 0 }
    };
    (b4) => {
        Position { row: 3, col: 1 }
    };
    (c4) => {
        Position { row: 3, col: 2 }
    };
    (d4) => {
        Position { row: 3, col: 3 }
    };
    (e4) => {
        Position { row: 3, col: 4 }
    };
    (f4) => {
        Position { row: 3, col: 5 }
    };
    (g4) => {
        Position { row: 3, col: 6 }
    };
    (h4) => {
        Position { row: 3, col: 7 }
    };
    (a5) => {
        Position { row: 4, col: 0 }
    };
    (b5) => {
        Position { row: 4, col: 1 }
    };
    (c5) => {
        Position { row: 4, col: 2 }
    };
    (d5) => {
        Position { row: 4, col: 3 }
    };
    (e5) => {
        Position { row: 4, col: 4 }
    };
    (f5) => {
        Position { row: 4, col: 5 }
    };
    (g5) => {
        Position { row: 4, col: 6 }
    };
    (h5) => {
        Position { row: 4, col: 7 }
    };
    (a6) => {
        Position { row: 5, col: 0 }
    };
    (b6) => {
        Position { row: 5, col: 1 }
    };
    (c6) => {
        Position { row: 5, col: 2 }
    };
    (d6) => {
        Position { row: 5, col: 3 }
    };
    (e6) => {
        Position { row: 5, col: 4 }
    };
    (f6) => {
        Position { row: 5, col: 5 }
    };
    (g6) => {
        Position { row: 5, col: 6 }
    };
    (h6) => {
        Position { row: 5, col: 7 }
    };
    (a7) => {
        Position { row: 6, col: 0 }
    };
    (b7) => {
        Position { row: 6, col: 1 }
    };
    (c7) => {
        Position { row: 6, col: 2 }
    };
    (d7) => {
        Position { row: 6, col: 3 }
    };
    (e7) => {
        Position { row: 6, col: 4 }
    };
    (f7) => {
        Position { row: 6, col: 5 }
    };
    (g7) => {
        Position { row: 6, col: 6 }
    };
    (h7) => {
        Position { row: 6, col: 7 }
    };
    (a8) => {
        Position { row: 7, col: 0 }
    };
    (b8) => {
        Position { row: 7, col: 1 }
    };
    (c8) => {
        Position { row: 7, col: 2 }
    };
    (d8) => {
        Position { row: 7, col: 3 }
    };
    (e8) => {
        Position { row: 7, col: 4 }
    };
    (f8) => {
        Position { row: 7, col: 5 }
    };
    (g8) => {
        Position { row: 7, col: 6 }
    };
    (h8) => {
        Position { row: 7, col: 7 }
    };

    ($rank:expr, $file:expr) => {
        Position {
            row: $rank,
            col: $file,
        }
    };
}

#[derive(Copy, Clone, PartialEq, Serialize)]
pub struct Move {
    pub source: Position,
    pub target: Position,
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.source, self.target)
    }
}

macro_rules! mv {
    ($src:ident, $tgt:ident) => {
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

#[derive(Copy, Clone, PartialEq, Serialize)]
pub enum MoveExtraInfo {
    Other,
    // Promotion(PieceType),
    Passed,
    EnPassant,
    // Castle,
}

#[derive(Copy, Clone, Serialize)]
pub struct MoveInfo {
    pub mv: Move,
    pub info: MoveExtraInfo,
}

#[derive(Copy, Clone, Serialize)]
pub struct Game {
    pub board: Board,
    pub player: Player,
    pub turn: u32,
    pub last_move: Option<MoveInfo>,
}
