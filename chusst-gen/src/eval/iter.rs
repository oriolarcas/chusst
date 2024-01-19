use crate::board::{
    Board, Direction, IterableBoard, ModifiableBoard, Piece, PieceType, Player, Position,
    PositionIterator,
};
use crate::eval::conditions::only_en_passant;
use crate::game::{CastlingRights, GameState, ModifiableGame};

#[macro_export]
macro_rules! dir {
    ($row:expr, $col:expr) => {
        Direction {
            row_inc: $row,
            col_inc: $col,
        }
    };
}
pub use dir;

struct PieceIterGameState<'a, B: Board> {
    game: &'a GameState<B>,
    position: Position,
}

pub enum PawnIterStates {
    Normal,
    Pass,
    CaptureLeft,
    CaptureRight,
    CaptureEnPassantLeft,
    CaptureEnPassantRight,
    End,
}

pub enum KnightIterStates {
    KnightIter0,
    KnightIter1,
    KnightIter2,
    KnightIter3,
    KnightIter4,
    KnightIter5,
    KnightIter6,
    KnightIter7,
    KnightIterEnd,
}

pub enum BishopIterStates {
    BishopIter0,
    BishopIter1,
    BishopIter2,
    BishopIter3,
    BishopIterEnd,
}

pub enum RookIterStates {
    RookIter0,
    RookIter1,
    RookIter2,
    RookIter3,
    RookIterEnd,
}

pub enum QueenIterStates {
    QueenIter0,
    QueenIter1,
    QueenIter2,
    QueenIter3,
    QueenIter4,
    QueenIter5,
    QueenIter6,
    QueenIter7,
    QueenIterEnd,
}

pub enum KingIterStates {
    KingIter0,
    KingIter1,
    KingIter2,
    KingIter3,
    KingIter4,
    KingIter5,
    KingIter6,
    KingIter7,
    KingIterKingsideCastle,
    KingIterQueensideCastle,
    KingIterEnd,
}

pub struct PositionalPieceIter<'a, B: Board, PieceStateEnum> {
    state: PieceStateEnum,
    game_state: PieceIterGameState<'a, B>,
}

pub struct RollingPieceIter<'a, B: Board, PieceStateEnum> {
    state: PieceStateEnum,
    game_state: PieceIterGameState<'a, B>,
    player: Player,
    iter: Option<Box<dyn Iterator<Item = Position> + 'a>>,
}

type PawnIter<'a, B> = PositionalPieceIter<'a, B, PawnIterStates>;
type KnightIter<'a, B> = PositionalPieceIter<'a, B, KnightIterStates>;
type BishopIter<'a, B> = RollingPieceIter<'a, B, BishopIterStates>;
type RookIter<'a, B> = RollingPieceIter<'a, B, RookIterStates>;
type QueenIter<'a, B> = RollingPieceIter<'a, B, QueenIterStates>;
type KingIter<'a, B> = PositionalPieceIter<'a, B, KingIterStates>;

pub enum PieceIter<'a, B: Board> {
    EmptySquare(std::iter::Empty<Position>),
    Pawn(PawnIter<'a, B>),
    Knight(KnightIter<'a, B>),
    Bishop(BishopIter<'a, B>),
    Rook(RookIter<'a, B>),
    Queen(QueenIter<'a, B>),
    King(KingIter<'a, B>),
}

pub fn piece_into_iter<B: Board>(
    game: &GameState<B>,
    position: Position,
) -> impl Iterator<Item = Position> + '_ {
    let Some(Piece { piece, player }) = game.at(&position) else {
        // println!("Square {} is empty", position);
        return PieceIter::EmptySquare(std::iter::empty());
    };

    let game_state = PieceIterGameState { game, position };

    match piece {
        PieceType::Pawn => PieceIter::Pawn(PawnIter {
            game_state,
            state: PawnIterStates::Normal,
        }),
        PieceType::Knight => PieceIter::Knight(KnightIter {
            game_state,
            state: KnightIterStates::KnightIter0,
        }),
        PieceType::Bishop => PieceIter::Bishop(BishopIter {
            game_state,
            state: BishopIterStates::BishopIter0,
            player,
            iter: None,
        }),
        PieceType::Rook => PieceIter::Rook(RookIter {
            game_state,
            state: RookIterStates::RookIter0,
            player,
            iter: None,
        }),
        PieceType::Queen => PieceIter::Queen(QueenIter {
            game_state,
            state: QueenIterStates::QueenIter0,
            player,
            iter: None,
        }),
        PieceType::King => PieceIter::King(KingIter {
            game_state,
            state: KingIterStates::KingIter0,
        }),
    }
}

impl<'a, B: Board> Iterator for PieceIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PieceIter::EmptySquare(iter) => iter.next(),
            PieceIter::Pawn(iter) => iter.next(),
            PieceIter::Knight(iter) => iter.next(),
            PieceIter::Bishop(iter) => iter.next(),
            PieceIter::Rook(iter) => iter.next(),
            PieceIter::Queen(iter) => iter.next(),
            PieceIter::King(iter) => iter.next(),
        }
    }
}

impl<'a, B: Board> Iterator for PawnIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        let square = self.game_state.game.at(&self.game_state.position);
        let player = &square.unwrap().player;
        let direction = B::pawn_progress_direction(player);
        let can_pass = match player {
            Player::White => self.game_state.position.rank == 1,
            Player::Black => self.game_state.position.rank == 6,
        };
        loop {
            let game = self.game_state.game;
            let result = match self.state {
                PawnIterStates::Normal => {
                    self.state = PawnIterStates::Pass;
                    game.try_move(&self.game_state.position, &dir!(direction, 0))
                        .only_empty()
                        .next()
                }
                PawnIterStates::Pass => {
                    self.state = PawnIterStates::CaptureLeft;
                    if can_pass
                        && game
                            .try_move(&self.game_state.position, &dir!(direction, 0))
                            .only_empty()
                            .next()
                            .is_some()
                    {
                        game.try_move(&self.game_state.position, &dir!(direction * 2, 0))
                            .only_empty()
                            .next()
                    } else {
                        None
                    }
                }
                PawnIterStates::CaptureLeft => {
                    self.state = PawnIterStates::CaptureRight;
                    game.try_move(&self.game_state.position, &dir!(direction, -1))
                        .only_enemy(*player)
                        .next()
                }
                PawnIterStates::CaptureRight => {
                    self.state = PawnIterStates::CaptureEnPassantLeft;
                    game.try_move(&self.game_state.position, &dir!(direction, 1))
                        .only_enemy(*player)
                        .next()
                }
                PawnIterStates::CaptureEnPassantLeft => {
                    self.state = PawnIterStates::CaptureEnPassantRight;
                    only_en_passant(
                        game,
                        game.try_move(&self.game_state.position, &dir!(direction, -1))
                            .next(),
                        direction,
                        player,
                        self.game_state.game.last_move(),
                    )
                }
                PawnIterStates::CaptureEnPassantRight => {
                    self.state = PawnIterStates::End;
                    only_en_passant(
                        game,
                        game.try_move(&self.game_state.position, &dir!(direction, 1))
                            .next(),
                        direction,
                        player,
                        self.game_state.game.last_move(),
                    )
                }
                PawnIterStates::End => return None,
            };

            if let Some(position) = result {
                return Some(position);
            }
        }
    }
}

macro_rules! positional_state {
    ($self:ident, $player:expr, $dir:expr => $next_state:expr) => {{
        $self.state = $next_state;
        $self
            .game_state
            .game
            .try_move(&$self.game_state.position, &$dir)
            .only_empty_or_enemy(*$player)
            .next()
    }};
}

macro_rules! walk_state {
    ($self:ident, $dir:expr => $next_state:expr) => {{
        let iter = if $self.iter.is_none() {
            $self.iter = Some(Box::new(
                $self
                    .game_state
                    .game
                    .direction_iterator(&$self.game_state.position, &$dir)
                    .take_while_empty_or_enemy($self.player),
            ));
            $self.iter.as_mut().unwrap()
        } else {
            $self.iter.as_mut().unwrap()
        };

        match iter.next() {
            Some(position) => Some(position),
            None => {
                $self.state = $next_state;
                $self.iter = None;
                None
            }
        }
    }};
}

impl<'a, B: Board> Iterator for KnightIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        let square = self.game_state.game.at(&self.game_state.position);
        let player = &square.unwrap().player;
        loop {
            let result = match self.state {
                KnightIterStates::KnightIter0 => {
                    positional_state!(self, player, dir!(-1, -2) => KnightIterStates::KnightIter1)
                }
                KnightIterStates::KnightIter1 => {
                    positional_state!(self, player, dir!(-1, 2) => KnightIterStates::KnightIter2)
                }
                KnightIterStates::KnightIter2 => {
                    positional_state!(self, player, dir!(-2, -1) => KnightIterStates::KnightIter3)
                }
                KnightIterStates::KnightIter3 => {
                    positional_state!(self, player, dir!(-2, 1) => KnightIterStates::KnightIter4)
                }
                KnightIterStates::KnightIter4 => {
                    positional_state!(self, player, dir!(2, -1) => KnightIterStates::KnightIter5)
                }
                KnightIterStates::KnightIter5 => {
                    positional_state!(self, player, dir!(2, 1) => KnightIterStates::KnightIter6)
                }
                KnightIterStates::KnightIter6 => {
                    positional_state!(self, player, dir!(1, -2) => KnightIterStates::KnightIter7)
                }
                KnightIterStates::KnightIter7 => {
                    positional_state!(self, player, dir!(1, 2) => KnightIterStates::KnightIterEnd)
                }
                KnightIterStates::KnightIterEnd => return None,
            };

            if let Some(position) = result {
                return Some(position);
            }
        }
    }
}

impl<'a, B: Board> Iterator for BishopIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let result = match self.state {
                BishopIterStates::BishopIter0 => {
                    walk_state!(self, dir!(-1, -1) => BishopIterStates::BishopIter1)
                }
                BishopIterStates::BishopIter1 => {
                    walk_state!(self, dir!(-1, 1) => BishopIterStates::BishopIter2)
                }
                BishopIterStates::BishopIter2 => {
                    walk_state!(self, dir!(1, -1) => BishopIterStates::BishopIter3)
                }
                BishopIterStates::BishopIter3 => {
                    walk_state!(self, dir!(1, 1) => BishopIterStates::BishopIterEnd)
                }
                BishopIterStates::BishopIterEnd => return None,
            };

            if let Some(position) = result {
                return Some(position);
            }
        }
    }
}

impl<'a, B: Board> Iterator for RookIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let result = match self.state {
                RookIterStates::RookIter0 => {
                    walk_state!(self, dir!(0, -1) => RookIterStates::RookIter1)
                }
                RookIterStates::RookIter1 => {
                    walk_state!(self, dir!(0, 1) => RookIterStates::RookIter2)
                }
                RookIterStates::RookIter2 => {
                    walk_state!(self, dir!(-1, 0) => RookIterStates::RookIter3)
                }
                RookIterStates::RookIter3 => {
                    walk_state!(self, dir!(1, 0) => RookIterStates::RookIterEnd)
                }
                RookIterStates::RookIterEnd => return None,
            };

            if let Some(position) = result {
                return Some(position);
            }
        }
    }
}

impl<'a, B: Board> Iterator for QueenIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let result = match self.state {
                QueenIterStates::QueenIter0 => {
                    walk_state!(self, dir!(-1, -1) => QueenIterStates::QueenIter1)
                }
                QueenIterStates::QueenIter1 => {
                    walk_state!(self, dir!(-1, 1) => QueenIterStates::QueenIter2)
                }
                QueenIterStates::QueenIter2 => {
                    walk_state!(self, dir!(1, -1) => QueenIterStates::QueenIter3)
                }
                QueenIterStates::QueenIter3 => {
                    walk_state!(self, dir!(1, 1) => QueenIterStates::QueenIter4)
                }
                QueenIterStates::QueenIter4 => {
                    walk_state!(self, dir!(0, -1) => QueenIterStates::QueenIter5)
                }
                QueenIterStates::QueenIter5 => {
                    walk_state!(self, dir!(0, 1) => QueenIterStates::QueenIter6)
                }
                QueenIterStates::QueenIter6 => {
                    walk_state!(self, dir!(-1, 0) => QueenIterStates::QueenIter7)
                }
                QueenIterStates::QueenIter7 => {
                    walk_state!(self, dir!(1, 0) => QueenIterStates::QueenIterEnd)
                }
                QueenIterStates::QueenIterEnd => return None,
            };

            if let Some(position) = result {
                return Some(position);
            }
        }
    }
}

impl<'a, B: Board> Iterator for KingIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        let square = self.game_state.game.at(&self.game_state.position);
        let player = &square.unwrap().player;
        let game = self.game_state.game;
        loop {
            let result = match self.state {
                KingIterStates::KingIter0 => {
                    positional_state!(self, player, dir!(-1, -1) => KingIterStates::KingIter1)
                }
                KingIterStates::KingIter1 => {
                    positional_state!(self, player, dir!(-1, 0) => KingIterStates::KingIter2)
                }
                KingIterStates::KingIter2 => {
                    positional_state!(self, player, dir!(-1, 1) => KingIterStates::KingIter3)
                }
                KingIterStates::KingIter3 => {
                    positional_state!(self, player, dir!(0, -1) => KingIterStates::KingIter4)
                }
                KingIterStates::KingIter4 => {
                    positional_state!(self, player, dir!(0, 1) => KingIterStates::KingIter5)
                }
                KingIterStates::KingIter5 => {
                    positional_state!(self, player, dir!(1, -1) => KingIterStates::KingIter6)
                }
                KingIterStates::KingIter6 => {
                    positional_state!(self, player, dir!(1, 0) => KingIterStates::KingIter7)
                }
                KingIterStates::KingIter7 => {
                    positional_state!(self, player, dir!(1, 1) => KingIterStates::KingIterKingsideCastle)
                }
                KingIterStates::KingIterKingsideCastle => {
                    self.state = KingIterStates::KingIterQueensideCastle;
                    if !self.game_state.game.can_castle_kingside(*player) {
                        None
                    } else if game
                        .try_move(&self.game_state.position, &dir!(0, 1))
                        .only_empty()
                        .next()
                        .is_some()
                    {
                        game.try_move(&self.game_state.position, &dir!(0, 2))
                            .only_empty()
                            .next()
                    } else {
                        None
                    }
                }
                KingIterStates::KingIterQueensideCastle => {
                    self.state = KingIterStates::KingIterEnd;
                    if !self.game_state.game.can_castle_queenside(*player) {
                        None
                    } else if game
                        .try_move(&self.game_state.position, &dir!(0, -1))
                        .only_empty()
                        .next()
                        .is_some()
                        && game
                            .try_move(&self.game_state.position, &dir!(0, -3))
                            .only_empty()
                            .next()
                            .is_some()
                    {
                        game.try_move(&self.game_state.position, &dir!(0, -2))
                            .only_empty()
                            .next()
                    } else {
                        None
                    }
                }
                KingIterStates::KingIterEnd => return None,
            };

            if let Some(position) = result {
                return Some(position);
            }
        }
    }
}
