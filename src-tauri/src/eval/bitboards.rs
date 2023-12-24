mod in_between;

use crate::board::{Board, PieceType, Player, Position};

type Bitboard = u64;

#[derive(Default)]
pub struct PlayerBitboards {
    pawns: Bitboard,
    knights: Bitboard,
    bishops: Bitboard,
    rooks: Bitboard,
    queens: Bitboard,
    kings: Bitboard,
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
    fn check_bitboard(bitboard: Bitboard, position: &Position) -> bool {
        let index = position.rank * 8 + position.file;
        bitboard & (1 << index) != 0
    }

    pub fn in_between(source: &Position, target: &Position) -> Bitboard {
        let source_index = source.rank * 8 + source.file;
        let target_index = target.rank * 8 + target.file;
        in_between::IN_BETWEEN_TABLE[source_index][target_index]
    }

    pub fn has_piece(&self, position: &Position, piece: &PieceType) -> bool {
        match piece {
            PieceType::Pawn => PlayerBitboards::check_bitboard(self.pawns, position),
            PieceType::Knight => PlayerBitboards::check_bitboard(self.knights, position),
            PieceType::Bishop => PlayerBitboards::check_bitboard(self.bishops, position),
            PieceType::Rook => PlayerBitboards::check_bitboard(self.rooks, position),
            PieceType::Queen => PlayerBitboards::check_bitboard(self.queens, position),
            PieceType::King => PlayerBitboards::check_bitboard(self.kings, position),
        }
    }

    pub fn into_iter(bitboard: Bitboard) -> BitboardIter {
        BitboardIter { bitboard }
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

pub struct Bitboards {
    white: PlayerBitboards,
    black: PlayerBitboards,
}

impl Bitboards {
    pub fn from(board: &Board) -> Self {
        let mut bitboards = Bitboards {
            white: PlayerBitboards::default(),
            black: PlayerBitboards::default(),
        };

        for rank in 0..8 {
            for file in 0..8 {
                let square = board.square(&Position { rank, file });
                if square.is_none() {
                    continue;
                }
                let square = square.unwrap();
                let player_bitboards = match square.player {
                    Player::White => &mut bitboards.white,
                    Player::Black => &mut bitboards.black,
                };

                let index = rank * 8 + file;

                match square.piece {
                    PieceType::Pawn => player_bitboards.pawns |= 1 << index,
                    PieceType::Knight => player_bitboards.knights |= 1 << index,
                    PieceType::Bishop => player_bitboards.bishops |= 1 << index,
                    PieceType::Rook => player_bitboards.rooks |= 1 << index,
                    PieceType::Queen => player_bitboards.queens |= 1 << index,
                    PieceType::King => player_bitboards.kings |= 1 << index,
                }
            }
        }

        bitboards
    }

    pub fn by_player(&self, player: &Player) -> &PlayerBitboards {
        match player {
            Player::White => &self.white,
            Player::Black => &self.black,
        }
    }
}
