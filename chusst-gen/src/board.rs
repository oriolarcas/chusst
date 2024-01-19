mod iter;

// Board representations
#[cfg(feature = "bitboards")]
mod bitboards;
#[cfg(feature = "compact-board")]
mod compact;
mod simple;

#[cfg(feature = "bitboards")]
pub use bitboards::Bitboards;
#[cfg(feature = "bitboards")]
pub(crate) use bitboards::PlayerBitboards;
#[cfg(feature = "compact-board")]
pub use compact::CompactBoard;
pub use simple::SimpleBoard;

pub use self::iter::{BoardIter, Direction, PositionIter, PositionIterator};

use self::iter::{try_move, DirectionIter};

use atty;
use colored::Colorize;
use serde::{ser::SerializeMap, ser::SerializeSeq, Serialize};
use std::{fmt, ops::Not};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize)]
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

impl Not for Player {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
    }
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

#[macro_export]
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

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct Position {
    pub rank: usize,
    pub file: usize,
}

impl Position {
    pub fn try_from_str(pos_str: &str) -> Option<Position> {
        if pos_str.len() != 2 {
            return None;
        }

        let mut chars = pos_str.chars();
        let file: usize = match chars.next()? {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            'd' => 3,
            'e' => 4,
            'f' => 5,
            'g' => 6,
            'h' => 7,
            _ => return None,
        };
        let rank_digit = usize::try_from(chars.next()?.to_digit(10)?).ok()?;
        let rank = match rank_digit {
            1..=8 => rank_digit - 1,
            _ => return None,
        };

        Some(Position { rank, file })
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let row = self.rank + 1;
        let col = ["a", "b", "c", "d", "e", "f", "g", "h"][self.file];
        write!(f, "{}{}", col, row)
    }
}

#[macro_export]
macro_rules! pos {
    (a1) => {
        Position { rank: 0, file: 0 }
    };
    (b1) => {
        Position { rank: 0, file: 1 }
    };
    (c1) => {
        Position { rank: 0, file: 2 }
    };
    (d1) => {
        Position { rank: 0, file: 3 }
    };
    (e1) => {
        Position { rank: 0, file: 4 }
    };
    (f1) => {
        Position { rank: 0, file: 5 }
    };
    (g1) => {
        Position { rank: 0, file: 6 }
    };
    (h1) => {
        Position { rank: 0, file: 7 }
    };
    (a2) => {
        Position { rank: 1, file: 0 }
    };
    (b2) => {
        Position { rank: 1, file: 1 }
    };
    (c2) => {
        Position { rank: 1, file: 2 }
    };
    (d2) => {
        Position { rank: 1, file: 3 }
    };
    (e2) => {
        Position { rank: 1, file: 4 }
    };
    (f2) => {
        Position { rank: 1, file: 5 }
    };
    (g2) => {
        Position { rank: 1, file: 6 }
    };
    (h2) => {
        Position { rank: 1, file: 7 }
    };
    (a3) => {
        Position { rank: 2, file: 0 }
    };
    (b3) => {
        Position { rank: 2, file: 1 }
    };
    (c3) => {
        Position { rank: 2, file: 2 }
    };
    (d3) => {
        Position { rank: 2, file: 3 }
    };
    (e3) => {
        Position { rank: 2, file: 4 }
    };
    (f3) => {
        Position { rank: 2, file: 5 }
    };
    (g3) => {
        Position { rank: 2, file: 6 }
    };
    (h3) => {
        Position { rank: 2, file: 7 }
    };
    (a4) => {
        Position { rank: 3, file: 0 }
    };
    (b4) => {
        Position { rank: 3, file: 1 }
    };
    (c4) => {
        Position { rank: 3, file: 2 }
    };
    (d4) => {
        Position { rank: 3, file: 3 }
    };
    (e4) => {
        Position { rank: 3, file: 4 }
    };
    (f4) => {
        Position { rank: 3, file: 5 }
    };
    (g4) => {
        Position { rank: 3, file: 6 }
    };
    (h4) => {
        Position { rank: 3, file: 7 }
    };
    (a5) => {
        Position { rank: 4, file: 0 }
    };
    (b5) => {
        Position { rank: 4, file: 1 }
    };
    (c5) => {
        Position { rank: 4, file: 2 }
    };
    (d5) => {
        Position { rank: 4, file: 3 }
    };
    (e5) => {
        Position { rank: 4, file: 4 }
    };
    (f5) => {
        Position { rank: 4, file: 5 }
    };
    (g5) => {
        Position { rank: 4, file: 6 }
    };
    (h5) => {
        Position { rank: 4, file: 7 }
    };
    (a6) => {
        Position { rank: 5, file: 0 }
    };
    (b6) => {
        Position { rank: 5, file: 1 }
    };
    (c6) => {
        Position { rank: 5, file: 2 }
    };
    (d6) => {
        Position { rank: 5, file: 3 }
    };
    (e6) => {
        Position { rank: 5, file: 4 }
    };
    (f6) => {
        Position { rank: 5, file: 5 }
    };
    (g6) => {
        Position { rank: 5, file: 6 }
    };
    (h6) => {
        Position { rank: 5, file: 7 }
    };
    (a7) => {
        Position { rank: 6, file: 0 }
    };
    (b7) => {
        Position { rank: 6, file: 1 }
    };
    (c7) => {
        Position { rank: 6, file: 2 }
    };
    (d7) => {
        Position { rank: 6, file: 3 }
    };
    (e7) => {
        Position { rank: 6, file: 4 }
    };
    (f7) => {
        Position { rank: 6, file: 5 }
    };
    (g7) => {
        Position { rank: 6, file: 6 }
    };
    (h7) => {
        Position { rank: 6, file: 7 }
    };
    (a8) => {
        Position { rank: 7, file: 0 }
    };
    (b8) => {
        Position { rank: 7, file: 1 }
    };
    (c8) => {
        Position { rank: 7, file: 2 }
    };
    (d8) => {
        Position { rank: 7, file: 3 }
    };
    (e8) => {
        Position { rank: 7, file: 4 }
    };
    (f8) => {
        Position { rank: 7, file: 5 }
    };
    (g8) => {
        Position { rank: 7, file: 6 }
    };
    (h8) => {
        Position { rank: 7, file: 7 }
    };

    ($rank:expr, $file:expr) => {
        Position {
            rank: $rank,
            file: $file,
        }
    };
}

pub type Files<T> = [T; 8];
pub type Ranks<T> = [Files<T>; 8];

pub trait ModifiableBoard<K, V>
where
    Self: Sized,
{
    fn at(&self, pos: &K) -> V;
    fn update(&mut self, pos: &K, value: V);
    fn move_piece(&mut self, source: &K, target: &K);
}

pub trait IterableBoard
where
    Self: Sized,
{
    fn try_move(&self, position: &Position, direction: &Direction) -> PositionIter<Self> {
        PositionIter::new(self, try_move(position, direction))
    }

    fn position_iter(&self, position: &Position) -> PositionIter<Self> {
        PositionIter::new(self, Some(*position))
    }

    fn board_iter(&self) -> BoardIter<Self> {
        BoardIter::new(self)
    }

    fn direction_iterator(
        &self,
        position: &Position,
        direction: &Direction,
    ) -> DirectionIter<Self> {
        DirectionIter::new(self, Some(*position), *direction)
    }
}

pub trait Board:
    ModifiableBoard<Position, Option<Piece>>
    + IterableBoard
    + Clone
    + Default
    + fmt::Debug
    + PartialEq
    + Serialize
    + fmt::Display
{
    const NEW_BOARD: Self;

    fn iter(&self) -> impl Iterator<Item = Position> {
        (0..8usize).flat_map(|rank| (0..8usize).map(move |file| Position { rank, file }))
    }

    fn home_rank(player: &Player) -> usize {
        match player {
            Player::White => 0,
            Player::Black => 7,
        }
    }

    fn can_promote_rank(player: &Player) -> usize {
        match player {
            Player::White => 6,
            Player::Black => 1,
        }
    }

    fn promotion_rank(player: &Player) -> usize {
        match player {
            Player::White => 7,
            Player::Black => 0,
        }
    }

    fn pawn_progress_direction(player: &Player) -> i8 {
        match player {
            Player::White => 1,
            Player::Black => -1,
        }
    }

    fn try_from_fen(fen: &str) -> Option<Self> {
        // Example initial
        // rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR

        let mut board: Self = Default::default();

        let ranks = fen.split('/').collect::<Vec<&str>>();

        if ranks.len() != 8 {
            return None;
        }

        for (rank, pieces) in ranks.iter().rev().enumerate() {
            if rank > 7 {
                return None;
            }

            let mut file: usize = 0;
            for piece_char in pieces.chars() {
                if file > 7 {
                    return None;
                }

                if piece_char.is_numeric() {
                    let skip = match piece_char.to_digit(10) {
                        Some(value) => match usize::try_from(value) {
                            Ok(usize_value) => usize_value,
                            Err(_) => return None,
                        },
                        None => return None,
                    };
                    file += skip;
                    continue;
                }

                let piece = match piece_char {
                    'r' => p!(rb),
                    'n' => p!(nb),
                    'b' => p!(bb),
                    'q' => p!(qb),
                    'k' => p!(kb),
                    'p' => p!(pb),
                    'R' => p!(rw),
                    'N' => p!(nw),
                    'B' => p!(bw),
                    'Q' => p!(qw),
                    'K' => p!(kw),
                    'P' => p!(pw),
                    _ => return None,
                };

                board.update(&pos!(rank, file), piece);

                file += 1;
            }

            if file != 8 {
                return None;
            }
        }

        Some(board)
    }
}

fn get_unicode_piece(piece: PieceType, player: Player) -> char {
    match (player, piece) {
        (Player::White, PieceType::Pawn) => '♙',
        (Player::White, PieceType::Knight) => '♘',
        (Player::White, PieceType::Bishop) => '♗',
        (Player::White, PieceType::Rook) => '♖',
        (Player::White, PieceType::Queen) => '♕',
        (Player::White, PieceType::King) => '♔',
        (Player::Black, PieceType::Pawn) => '♟',
        (Player::Black, PieceType::Knight) => '♞',
        (Player::Black, PieceType::Bishop) => '♝',
        (Player::Black, PieceType::Rook) => '♜',
        (Player::Black, PieceType::Queen) => '♛',
        (Player::Black, PieceType::King) => '♚',
    }
}

fn format_board(board: &impl Board, f: &mut fmt::Formatter) -> fmt::Result {
    let mut rows: Vec<String> = Default::default();

    let square_dark = |rank: usize, file: usize| -> bool { (rank + file) % 2 == 0 };

    let is_atty = atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stderr);
    let (left_square, right_square) = if is_atty { (" ", " ") } else { ("[", "]") };

    rows.push("   a  b  c  d  e  f  g  h ".to_owned());
    for rank in (0..8).rev() {
        let mut row_str = format!("{} ", rank + 1);
        for file in 0..8 {
            let piece = board.at(&pos!(rank, file)).map(|square_value| {
                (
                    get_unicode_piece(square_value.piece, square_value.player),
                    square_value.player,
                )
            });

            let is_dark_square = square_dark(rank, file);

            let colored_piece_str = match piece {
                Some((piece_symbol, player)) => {
                    let piece_str = format!("{}{}{}", left_square, piece_symbol, right_square);
                    match player {
                        Player::White => piece_str.black(),
                        Player::Black => piece_str.red(),
                    }
                }
                None => format!("{} {}", left_square, right_square).normal(),
            };

            let colored_square_str = if is_dark_square {
                colored_piece_str.on_black()
            } else {
                colored_piece_str.on_white()
            };

            let square_str = if !is_atty {
                colored_square_str.clear()
            } else {
                colored_square_str
            };

            row_str += format!("{}", square_str).as_str();
        }
        rows.push(row_str);
    }

    writeln!(f, "{}", rows.join("\n"))
}

struct SerializableBoardRanks<'a, B: Board> {
    board: &'a B,
}

impl<'a, B: Board> Serialize for SerializableBoardRanks<'a, B> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(8))?;
        for rank in 0..8usize {
            let rank_of_squares = (0..8usize).map(|file| self.board.at(&pos!(rank, file)));
            seq.serialize_element(&rank_of_squares.collect::<Vec<Option<Piece>>>())?;
        }
        seq.end()
    }
}

fn serialize_board<S>(board: &impl Board, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut map = serializer.serialize_map(Some(1))?;
    map.serialize_entry("ranks", &SerializableBoardRanks { board })?;
    map.end()
}
