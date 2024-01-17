use crate::board::{Board, Piece, PieceType, Player, Position};
use crate::eval::conditions::{
    only_empty, only_empty_or_enemy, only_en_passant, only_enemy, try_move, Direction,
};
use crate::game::GameState;
use crate::pos;

pub struct BoardIter {
    position: Position,
}

impl Default for BoardIter {
    fn default() -> Self {
        BoardIter {
            position: pos!(0, 0),
        }
    }
}

impl Iterator for BoardIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position.rank > 7 {
            return None;
        }

        let current_position = self.position;

        if self.position.file == 7 {
            self.position.rank += 1;
            self.position.file = 0;
        } else {
            self.position.file += 1;
        }

        Some(current_position)
    }
}

pub struct PlayerPiecesIter<'a, B: Board> {
    pub(super) board: &'a B,
    pub(super) player: &'a Player,
    pub(super) board_iter: BoardIter,
}

macro_rules! player_pieces_iter {
    (board: $board:expr, player: $player:expr) => {
        PlayerPiecesIter {
            board: $board,
            player: $player,
            board_iter: Default::default(),
        }
    };
}
pub(crate) use player_pieces_iter;

impl<'a, B: Board> Iterator for PlayerPiecesIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        for position in self.board_iter.by_ref() {
            if let Some(Piece { piece: _, player }) = self.board.at(&position) {
                if player == *self.player {
                    return Some(position);
                }
            }
        }
        None
    }
}

macro_rules! dir {
    ($row:expr, $col:expr) => {
        Direction {
            row_inc: $row,
            col_inc: $col,
        }
    };
}
pub(crate) use dir;

struct WalkPath {
    position: Position,
    direction: Direction,
    stop_walking: bool,
}

trait BoardIterator {
    fn walk(position: Position, direction: Direction) -> WalkPath;
    fn next(&mut self, board: &impl Board, player: &Player) -> Option<Position>;
}

impl BoardIterator for WalkPath {
    fn walk(position: Position, direction: Direction) -> WalkPath {
        WalkPath {
            position,
            direction,
            stop_walking: false,
        }
    }

    fn next(&mut self, board: &impl Board, player: &Player) -> Option<Position> {
        if self.stop_walking {
            return None;
        }
        match only_empty_or_enemy(board, try_move(&self.position, &self.direction), player) {
            Some(new_position) => {
                self.position = new_position;
                match only_empty(board, Some(new_position)) {
                    Some(_) => Some(new_position),
                    None => {
                        self.stop_walking = true;
                        Some(new_position)
                    }
                }
            }
            None => None,
        }
    }
}

pub struct BoardIteratorAdapter<'a, B: Board> {
    board: &'a B,
    player: Player,
    walker: WalkPath,
}

impl<'a, B: Board> Iterator for BoardIteratorAdapter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        self.walker.next(self.board, &self.player)
    }
}

pub fn into_rolling_board_iterator<'a>(
    board: &'a impl Board,
    player: &Player,
    position: &Position,
    direction: &Direction,
) -> impl Iterator<Item = Position> + 'a {
    BoardIteratorAdapter {
        board,
        player: *player,
        walker: <WalkPath as BoardIterator>::walk(*position, *direction),
    }
}

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
    walker: Option<WalkPath>,
}

impl<'a, B: Board, P> RollingPieceIter<'a, B, P> {
    fn walk(&'a self, direction: Direction) -> WalkPath {
        WalkPath {
            position: self.game_state.position,
            direction,
            stop_walking: false,
        }
    }
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
    let Some(Piece { piece, player }) = game.board.at(&position) else {
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
            walker: None,
        }),
        PieceType::Rook => PieceIter::Rook(RookIter {
            game_state,
            state: RookIterStates::RookIter0,
            player,
            walker: None,
        }),
        PieceType::Queen => PieceIter::Queen(QueenIter {
            game_state,
            state: QueenIterStates::QueenIter0,
            player,
            walker: None,
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
        let square = self.game_state.game.board.at(&self.game_state.position);
        let player = &square.unwrap().player;
        let direction = B::pawn_progress_direction(player);
        let can_pass = match player {
            Player::White => self.game_state.position.rank == 1,
            Player::Black => self.game_state.position.rank == 6,
        };
        loop {
            let result = match self.state {
                PawnIterStates::Normal => {
                    self.state = PawnIterStates::Pass;
                    only_empty(
                        &self.game_state.game.board,
                        try_move(&self.game_state.position, &dir!(direction, 0)),
                    )
                }
                PawnIterStates::Pass => {
                    self.state = PawnIterStates::CaptureLeft;
                    if can_pass
                        && only_empty(
                            &self.game_state.game.board,
                            try_move(&self.game_state.position, &dir!(direction, 0)),
                        )
                        .is_some()
                    {
                        only_empty(
                            &self.game_state.game.board,
                            try_move(&self.game_state.position, &dir!(direction * 2, 0)),
                        )
                    } else {
                        None
                    }
                }
                PawnIterStates::CaptureLeft => {
                    self.state = PawnIterStates::CaptureRight;
                    only_enemy(
                        &self.game_state.game.board,
                        try_move(&self.game_state.position, &dir!(direction, -1)),
                        player,
                    )
                }
                PawnIterStates::CaptureRight => {
                    self.state = PawnIterStates::CaptureEnPassantLeft;
                    only_enemy(
                        &self.game_state.game.board,
                        try_move(&self.game_state.position, &dir!(direction, 1)),
                        player,
                    )
                }
                PawnIterStates::CaptureEnPassantLeft => {
                    self.state = PawnIterStates::CaptureEnPassantRight;
                    only_en_passant(
                        &self.game_state.game.board,
                        &self.game_state.game.last_move,
                        try_move(&self.game_state.position, &dir!(direction, -1)),
                        player,
                        direction,
                    )
                }
                PawnIterStates::CaptureEnPassantRight => {
                    self.state = PawnIterStates::End;
                    only_en_passant(
                        &self.game_state.game.board,
                        &self.game_state.game.last_move,
                        try_move(&self.game_state.position, &dir!(direction, 1)),
                        player,
                        direction,
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
        only_empty_or_enemy(
            &$self.game_state.game.board,
            try_move(&$self.game_state.position, &$dir),
            $player,
        )
    }};
}

macro_rules! walk_state {
    ($self:ident, $dir:expr => $next_state:expr) => {{
        if $self.walker.is_none() {
            $self.walker = Some($self.walk($dir));
        }

        match $self
            .walker
            .as_mut()
            .unwrap()
            .next(&$self.game_state.game.board, &$self.player)
        {
            Some(position) => Some(position),
            None => {
                $self.state = $next_state;
                $self.walker = None;
                None
            }
        }
    }};
}

impl<'a, B: Board> Iterator for KnightIter<'a, B> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        let square = self.game_state.game.board.at(&self.game_state.position);
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
        let square = self.game_state.game.board.at(&self.game_state.position);
        let player = &square.unwrap().player;
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
                    if !self.game_state.game.info.can_castle_kingside(player) {
                        None
                    } else if only_empty(
                        &self.game_state.game.board,
                        try_move(&self.game_state.position, &dir!(0, 1)),
                    )
                    .is_some()
                    {
                        only_empty(
                            &self.game_state.game.board,
                            try_move(&self.game_state.position, &dir!(0, 2)),
                        )
                    } else {
                        None
                    }
                }
                KingIterStates::KingIterQueensideCastle => {
                    self.state = KingIterStates::KingIterEnd;
                    if !self.game_state.game.info.can_castle_queenside(player) {
                        None
                    } else if only_empty(
                        &self.game_state.game.board,
                        try_move(&self.game_state.position, &dir!(0, -1)),
                    )
                    .is_some()
                        && only_empty(
                            &self.game_state.game.board,
                            try_move(&self.game_state.position, &dir!(0, -3)),
                        )
                        .is_some()
                    {
                        only_empty(
                            &self.game_state.game.board,
                            try_move(&self.game_state.position, &dir!(0, -2)),
                        )
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
