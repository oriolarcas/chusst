use atty;
use colored::Colorize;
use serde::Serialize;
use std::fmt;

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

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    fn try_from_str(pos_str: &str) -> Option<Position> {
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

        Some(Position {
            row: rank,
            col: file,
        })
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let row = self.row + 1;
        let col = ["a", "b", "c", "d", "e", "f", "g", "h"][self.col];
        write!(f, "{}{}", col, row)
    }
}

#[macro_export]
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

pub type Square = Option<Piece>;

pub type Rank<T> = [T; 8];
pub type Row<T> = Rank<T>;
pub type Rows<T> = Rank<Row<T>>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize)]
pub struct Board {
    // rows[x][y], where x = 0..7 = rows 1..8, and y = 0..7 = columns a..h
    // for instance, e4 is Board.rows[2][4]
    pub(self) rows: Rows<Square>,
}

impl Board {
    pub fn try_from_fen(fen: &str) -> Option<Board> {
        // Example initial
        // rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR

        let mut board: Board = Default::default();

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

    pub fn square(&self, pos: &Position) -> &Square {
        &self.rows[pos.row][pos.col]
    }

    pub fn update(&mut self, pos: &Position, value: Square) {
        self.rows[pos.row][pos.col] = value;
    }

    pub fn move_piece(&mut self, source: &Position, target: &Position) {
        self.rows[target.row][target.col] = self.rows[source.row][source.col];
        self.rows[source.row][source.col] = None;
    }

    pub fn home_rank(player: &Player) -> usize {
        match player {
            Player::White => 0,
            Player::Black => 7,
        }
    }

    pub fn promotion_rank(player: &Player) -> usize {
        match player {
            Player::White => 7,
            Player::Black => 0,
        }
    }

    pub fn pawn_progress_direction(player: &Player) -> i8 {
        match player {
            Player::White => 1,
            Player::Black => -1,
        }
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

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rows: Vec<String> = Default::default();

        let square_dark = |rank: usize, file: usize| -> bool { (rank + file) % 2 == 0 };

        let is_atty = atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stderr);
        let (left_square, right_square) = if is_atty { (" ", " ") } else { ("[", "]") };

        rows.push("   a  b  c  d  e  f  g  h ".to_owned());
        for (rank, row) in self.rows.iter().rev().enumerate() {
            let mut row_str = String::from(format!("{} ", 8 - rank));
            for (file, square) in row.iter().enumerate() {
                let piece = match square {
                    Some(square_value) => Some((
                        get_unicode_piece(square_value.piece, square_value.player),
                        square_value.player,
                    )),
                    None => None,
                };

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
}

const INITIAL_BOARD: Board = Board {
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
};

impl Board {
    pub const fn new() -> Board {
        INITIAL_BOARD
    }
}

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

            if en_passant_pos.row != passed_rank {
                return None;
            }

            Some(MoveInfo {
                mv: mv!(
                    pos!(source_rank, en_passant_pos.col),
                    pos!(target_rank, en_passant_pos.col)
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
