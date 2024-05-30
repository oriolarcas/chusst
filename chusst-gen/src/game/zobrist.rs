use crate::game::Position;
use lazy_static::lazy_static;
use rand::prelude::*;
use std::hash::{BuildHasher, Hasher};
use std::ops::BitXorAssign;
use std::{fmt, sync::Mutex};

use crate::{
    board::{Board, Piece, PieceType, Player},
    pos,
};

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ZobristHash(u64);

impl BitXorAssign for ZobristHash {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl fmt::Display for ZobristHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl From<ZobristHash> for u64 {
    fn from(value: ZobristHash) -> u64 {
        value.0
    }
}

impl From<u64> for ZobristHash {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Default)]
pub struct ZobristHasher(ZobristHash);

/// Custom Hasher for ZobristHasher that just uses the hash value unchanged,
/// to avoid hashing on top of the Zobrist hash.
impl Hasher for ZobristHasher {
    fn finish(&self) -> u64 {
        self.0.into()
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!()
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = ZobristHash::from(i);
    }
}

#[derive(Default)]
pub struct ZobristHashBuilder;

impl BuildHasher for ZobristHashBuilder {
    type Hasher = ZobristHasher;

    fn build_hasher(&self) -> Self::Hasher {
        ZobristHasher::default()
    }
}

#[derive(Copy, Clone)]
struct RandomHash(ZobristHash);

impl Default for RandomHash {
    fn default() -> Self {
        Self(ZobristHash(RNG.lock().unwrap().gen()))
    }
}

impl From<RandomHash> for ZobristHash {
    fn from(value: RandomHash) -> Self {
        value.0
    }
}

#[derive(Default)]
struct PiecesHash {
    pawn: RandomHash,
    knight: RandomHash,
    bishop: RandomHash,
    rook: RandomHash,
    queen: RandomHash,
    king: RandomHash,
}

impl PiecesHash {
    fn by_piece(&self, piece: PieceType) -> ZobristHash {
        match piece {
            PieceType::Pawn => self.pawn,
            PieceType::Knight => self.knight,
            PieceType::Bishop => self.bishop,
            PieceType::Rook => self.rook,
            PieceType::Queen => self.queen,
            PieceType::King => self.king,
        }
        .into()
    }
}

#[derive(Default)]
struct CastlingRightsHash {
    does_not_have_kingside_hash: RandomHash,
    does_not_have_queenside_hash: RandomHash,
}

#[derive(Default)]
struct ByPlayer<T> {
    white: T,
    black: T,
}

impl<T> ByPlayer<T> {
    fn by_player(&self, player: Player) -> &T {
        match player {
            Player::White => &self.white,
            Player::Black => &self.black,
        }
    }
}

#[derive(Default)]
struct ByFile<T>([T; 8]);

impl<T> ByFile<T> {
    fn at(&self, file: usize) -> &T {
        &self.0[file]
    }
}

#[derive(Default)]
struct ByPosition<T>([ByFile<T>; 8]);

impl<T> ByPosition<T> {
    fn at(&self, rank: usize, file: usize) -> &T {
        self.0[rank].at(file)
    }
}

#[derive(Default)]
struct RandomTable {
    pieces: ByPosition<ByPlayer<PiecesHash>>,
    black_turn: RandomHash,
    castling: ByPlayer<CastlingRightsHash>,
    can_do_en_passant: ByFile<RandomHash>,
}

lazy_static! {
    // Deterministic random number generator.
    static ref RNG: Mutex<StdRng> = Mutex::new(StdRng::seed_from_u64(0));
    static ref RANDOM_TABLE: RandomTable = RandomTable::default();
}

impl ZobristHash {
    pub fn switch_turn(&mut self) {
        self.0 ^= RANDOM_TABLE.black_turn.0 .0;
    }

    pub fn switch_kingside_castling(&mut self, player: Player) {
        self.0 ^= RANDOM_TABLE
            .castling
            .by_player(player)
            .does_not_have_kingside_hash
            .0
             .0;
    }

    pub fn switch_queenside_castling(&mut self, player: Player) {
        self.0 ^= RANDOM_TABLE
            .castling
            .by_player(player)
            .does_not_have_queenside_hash
            .0
             .0;
    }

    pub fn switch_en_passant_file(&mut self, file: usize) {
        self.0 ^= RANDOM_TABLE.can_do_en_passant.at(file).0 .0;
    }

    pub fn update_piece(
        &mut self,
        position: &Position,
        old_piece: Option<Piece>,
        new_piece: Option<Piece>,
    ) {
        // First remove the old piece
        if let Some(Piece { piece, player }) = old_piece {
            self.0 ^= RANDOM_TABLE
                .pieces
                .at(position.rank, position.file)
                .by_player(player)
                .by_piece(piece)
                .0;
        }
        // Then add the new piece
        if let Some(Piece { piece, player }) = new_piece {
            self.0 ^= RANDOM_TABLE
                .pieces
                .at(position.rank, position.file)
                .by_player(player)
                .by_piece(piece)
                .0;
        }
    }

    pub fn move_piece(&mut self, source: &Position, target: &Position, moved_piece: Piece) {
        // First remove the old piece
        self.0 ^= RANDOM_TABLE
            .pieces
            .at(source.rank, source.file)
            .by_player(moved_piece.player)
            .by_piece(moved_piece.piece)
            .0;
        // Then add the new piece
        self.0 ^= RANDOM_TABLE
            .pieces
            .at(target.rank, target.file)
            .by_player(moved_piece.player)
            .by_piece(moved_piece.piece)
            .0;
    }
}

impl<B: Board> From<&B> for ZobristHash {
    fn from(value: &B) -> Self {
        let mut hash = ZobristHash(0);
        for rank in 0..8usize {
            for file in 0..8usize {
                if let Some(Piece { piece, player }) = value.at(&pos!(rank, file)) {
                    hash ^= RANDOM_TABLE
                        .pieces
                        .at(rank, file)
                        .by_player(player)
                        .by_piece(piece);
                }
            }
        }

        hash
    }
}
