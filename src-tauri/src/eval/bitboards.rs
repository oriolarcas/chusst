mod in_between;

use crate::board::{Board, ModifiableBoard, Piece, PieceType, Player, Position, Square};
use crate::game::Game;

use std::fmt;
use std::ops::Index;

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

#[derive(Clone)]
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

    pub fn new(player: Player) -> PlayerBitboards {
        PlayerBitboards {
            player,
            pawns: Default::default(),
            knights: Default::default(),
            bishops: Default::default(),
            rooks: Default::default(),
            queens: Default::default(),
            kings: Default::default(),
            combined: Default::default(),
        }
    }

    pub fn in_between(source: &Position, target: &Position) -> Bitboard {
        let source_index = position_to_bitboard_index(source);
        let target_index = position_to_bitboard_index(target);
        in_between::IN_BETWEEN_TABLE[source_index][target_index]
    }

    pub fn has_position(&self, position: &Position) -> bool {
        check_bitboard(self.combined(), position)
    }

    pub fn has_piece(&self, position: &Position, piece: &PieceType) -> bool {
        match piece {
            PieceType::Pawn => check_bitboard(self.pawns, position),
            PieceType::Knight => check_bitboard(self.knights, position),
            PieceType::Bishop => check_bitboard(self.bishops, position),
            PieceType::Rook => check_bitboard(self.rooks, position),
            PieceType::Queen => check_bitboard(self.queens, position),
            PieceType::King => check_bitboard(self.kings, position),
        }
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
}

impl Index<Position> for PlayerBitboards {
    type Output = Option<PieceType>;

    fn index(&self, index: Position) -> &Self::Output {
        let position_mask = bitboard_from_position(&index);
        if self.pawns & position_mask != 0 {
            &Some(PieceType::Pawn)
        } else if self.knights & position_mask != 0 {
            &Some(PieceType::Knight)
        } else if self.bishops & position_mask != 0 {
            &Some(PieceType::Bishop)
        } else if self.rooks & position_mask != 0 {
            &Some(PieceType::Rook)
        } else if self.queens & position_mask != 0 {
            &Some(PieceType::Queen)
        } else if self.kings & position_mask != 0 {
            &Some(PieceType::King)
        } else {
            &None
        }
    }
}

impl ModifiableBoard for PlayerBitboards {
    fn update(&mut self, pos: &Position, value: Square) {
        let mask = bitboard_from_position(pos);
        match value {
            Some(piece) => {
                match piece.piece {
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
        let mut board = Board::default();

        for rank in 0..8 {
            for file in 0..8 {
                let pos = Position { rank, file };
                board.update(
                    &pos,
                    self[pos].map(|piece| Piece {
                        piece,
                        player: self.player,
                    }),
                );
            }
        }

        write!(f, "{}", board)
    }
}

#[derive(Clone)]
pub struct BitboardGame {
    game: Game,
    white: PlayerBitboards,
    black: PlayerBitboards,
}

impl<'a> BitboardGame {
    pub fn by_player(&self, player: &Player) -> &PlayerBitboards {
        match player {
            Player::White => &self.white,
            Player::Black => &self.black,
        }
    }

    pub fn as_ref(&self) -> &Game {
        &self.game
    }

    pub fn as_mut(&mut self) -> &mut Game {
        &mut self.game
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

    fn at(&self, index: &Position) -> Square {
        if let Some(piece) = self.white[*index] {
            Some(Piece {
                player: Player::White,
                piece,
            })
        } else if let Some(piece) = self.black[*index] {
            Some(Piece {
                player: Player::Black,
                piece,
            })
        } else {
            None
        }
    }
}

impl From<&Game> for BitboardGame {
    fn from(game: &Game) -> BitboardGame {
        let mut bitboards = BitboardGame {
            game: game.clone(),
            white: PlayerBitboards::new(Player::White),
            black: PlayerBitboards::new(Player::Black),
        };

        for rank in 0..8 {
            for file in 0..8 {
                let square = game.board.square(&Position { rank, file });
                if square.is_none() {
                    continue;
                }
                let square = square.unwrap();
                let player_bitboards = match square.player {
                    Player::White => &mut bitboards.white,
                    Player::Black => &mut bitboards.black,
                };

                let piece_mask = bitboard_from_rank_and_file(rank, file);

                match square.piece {
                    PieceType::Pawn => player_bitboards.pawns |= piece_mask,
                    PieceType::Knight => player_bitboards.knights |= piece_mask,
                    PieceType::Bishop => player_bitboards.bishops |= piece_mask,
                    PieceType::Rook => player_bitboards.rooks |= piece_mask,
                    PieceType::Queen => player_bitboards.queens |= piece_mask,
                    PieceType::King => player_bitboards.kings |= piece_mask,
                }

                player_bitboards.combined |= piece_mask;
            }
        }

        bitboards
    }
}

impl ModifiableBoard for BitboardGame {
    fn update(&mut self, pos: &Position, value: Square) {
        let value_player = value.map(|piece| piece.player);
        let player_at_position = self.player_at_position(pos);

        match value_player {
            Some(player) => match player {
                Player::White => {
                    if Some(Player::Black) == player_at_position {
                        self.black.update(pos, None);
                    }
                    self.white.update(pos, value);
                }
                Player::Black => {
                    if Some(Player::White) == player_at_position {
                        self.white.update(pos, None);
                    }
                    self.black.update(pos, value);
                }
            },
            None => {
                if Some(Player::White) == player_at_position {
                    self.white.update(pos, value);
                }
                if Some(Player::Black) == player_at_position {
                    self.black.update(pos, value);
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

impl fmt::Display for BitboardGame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut board = Board::default();

        for rank in 0..8 {
            for file in 0..8 {
                let pos = Position { rank, file };
                board.update(&pos, self.at(&pos));
            }
        }

        write!(f, "{}", board)
    }
}
