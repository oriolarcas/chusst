use crate::board::{Board, MoveExtraInfo, MoveInfo, PieceType, Player, Position};

pub struct Direction {
    pub row_inc: i8,
    pub col_inc: i8,
}

macro_rules! dir {
    ($row:expr, $col:expr) => {
        Direction {
            row_inc: $row,
            col_inc: $col,
        }
    };
}

struct WalkPath {
    position: Position,
    direction: Direction,
    stop_walking: bool,
}

trait BoardIterator {
    fn next(&mut self, board: &Board, player: &Player) -> Option<Position>;
}

impl BoardIterator for WalkPath {
    fn next(&mut self, board: &Board, player: &Player) -> Option<Position> {
        if self.stop_walking {
            return None;
        }
        match only_empty_or_enemy(&board, try_move(&self.position, &self.direction), player) {
            Some(new_position) => {
                self.position = new_position;
                match only_empty(&board, Some(new_position)) {
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

pub fn try_move(position: &Position, direction: &Direction) -> Option<Position> {
    let row = i8::try_from(position.row).unwrap() + direction.row_inc;
    let col = i8::try_from(position.col).unwrap() + direction.col_inc;

    if row < 0 || row >= 8 {
        return None;
    }

    if col < 0 || col >= 8 {
        return None;
    }

    match (usize::try_from(row), usize::try_from(col)) {
        (Ok(urow), Ok(ucol)) => Some(pos!(urow, ucol)),
        _ => None,
    }
}

fn only_empty(board: &Board, position: Option<Position>) -> Option<Position> {
    match board.square(position?) {
        Some(_) => None,
        None => position,
    }
}

fn only_player(board: &Board, position: Option<Position>, player: Player) -> Option<Position> {
    match board.square(position?) {
        Some(square) => {
            if square.player == player {
                position
            } else {
                None
            }
        }
        None => None,
    }
}

pub fn enemy(player: &Player) -> Player {
    match player {
        Player::White => Player::Black,
        Player::Black => Player::White,
    }
}

pub fn only_enemy(board: &Board, position: Option<Position>, player: &Player) -> Option<Position> {
    only_player(board, position, enemy(player))
}

fn only_empty_or_enemy(
    board: &Board,
    position: Option<Position>,
    player: &Player,
) -> Option<Position> {
    match position {
        Some(position_value) => match board.square(position_value) {
            // Occupied square
            Some(piece) => {
                if piece.player != *player {
                    // Enemy piece
                    Some(position_value)
                } else {
                    // Friend piece
                    None
                }
            }
            // Empty square
            None => Some(position_value),
        },
        None => None,
    }
}

fn only_en_passant(
    board: &Board,
    last_move: &Option<MoveInfo>,
    position: Option<Position>,
    player: &Player,
    direction: i8,
) -> Option<Position> {
    match only_empty(board, position) {
        Some(position_value) => {
            let reverse_direction = Direction {
                row_inc: -direction,
                col_inc: 0,
            };

            match (
                last_move,
                only_enemy(board, try_move(&position_value, &reverse_direction), player),
            ) {
                (Some(last_move_info), Some(passed_position)) => {
                    if passed_position == last_move_info.mv.target
                        && last_move_info.info == MoveExtraInfo::Passed
                    {
                        position
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        None => None,
    }
}

struct PieceIterBoardState<'a> {
    board: &'a Board,
    last_move: &'a Option<MoveInfo>,
    position: Position,
}

pub enum PawnIterStates {
    PawnIterNormal,
    PawnIterPass,
    PawnIterCaptureLeft,
    PawnIterCaptureRight,
    PawnIterCaptureEnPassantLeft,
    PawnIterCaptureEnPassantRight,
    PawnIterEnd,
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

pub struct PositionalPieceIter<'a, PieceStateEnum> {
    state: PieceStateEnum,
    board_state: PieceIterBoardState<'a>,
}

pub struct RollingPieceIter<'a, PieceStateEnum> {
    state: PieceStateEnum,
    board_state: PieceIterBoardState<'a>,
    player: Player,
    walker: Option<WalkPath>,
}

impl<'a, P> RollingPieceIter<'a, P> {
    fn walk(&'a self, direction: Direction) -> WalkPath {
        WalkPath {
            position: self.board_state.position,
            direction,
            stop_walking: false,
        }
    }
}

type PawnIter<'a> = PositionalPieceIter<'a, PawnIterStates>;
type KnightIter<'a> = PositionalPieceIter<'a, KnightIterStates>;
type BishopIter<'a> = RollingPieceIter<'a, BishopIterStates>;

pub enum PieceIter<'a> {
    EmptySquareIterType(std::iter::Empty<Position>),
    PawnIterType(PawnIter<'a>),
    KnightIterType(KnightIter<'a>),
    BishopIterType(BishopIter<'a>),
}

pub fn piece_into_iter<'a>(
    board: &'a Board,
    last_move: &'a Option<MoveInfo>,
    position: Position,
) -> impl Iterator<Item = Position> + 'a {
    let square = board.square(position);
    if square.is_none() {
        // println!("Square {} is empty", position);
        return PieceIter::EmptySquareIterType(std::iter::empty());
    }

    let piece = &square.unwrap().piece;
    let player = &square.unwrap().player;

    let board_state = PieceIterBoardState {
        board: &board,
        last_move: &last_move,
        position,
    };

    match piece {
        PieceType::Pawn => PieceIter::PawnIterType(PawnIter {
            board_state,
            state: PawnIterStates::PawnIterNormal,
        }),
        PieceType::Knight => PieceIter::KnightIterType(KnightIter {
            board_state,
            state: KnightIterStates::KnightIter0,
        }),
        PieceType::Bishop => PieceIter::BishopIterType(BishopIter {
            board_state,
            state: BishopIterStates::BishopIter0,
            player: *player,
            walker: None,
        }),
        PieceType::Rook => todo!(),
        PieceType::Queen => todo!(),
        PieceType::King => todo!(),
    }
}

impl<'a> Iterator for PieceIter<'a> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PieceIter::EmptySquareIterType(iter) => iter.next(),
            PieceIter::PawnIterType(iter) => iter.next(),
            PieceIter::KnightIterType(iter) => iter.next(),
            PieceIter::BishopIterType(iter) => iter.next(),
        }
    }
}

impl<'a> Iterator for PawnIter<'a> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        let square = self.board_state.board.square(self.board_state.position);
        let player = &square.unwrap().player;
        let direction = match player {
            Player::White => 1,
            Player::Black => -1,
        };
        let can_pass = match player {
            Player::White => self.board_state.position.row == 1,
            Player::Black => self.board_state.position.row == 6,
        };
        loop {
            let result = match self.state {
                PawnIterStates::PawnIterNormal => {
                    self.state = PawnIterStates::PawnIterPass;
                    only_empty(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(direction, 0)),
                    )
                }
                PawnIterStates::PawnIterPass => {
                    self.state = PawnIterStates::PawnIterCaptureLeft;
                    if can_pass
                        && try_move(&self.board_state.position, &dir!(direction, 0)).is_some()
                    {
                        only_empty(
                            &self.board_state.board,
                            try_move(&self.board_state.position, &dir!(direction * 2, 0)),
                        )
                    } else {
                        None
                    }
                }
                PawnIterStates::PawnIterCaptureLeft => {
                    self.state = PawnIterStates::PawnIterCaptureRight;
                    only_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(direction, -1)),
                        player,
                    )
                }
                PawnIterStates::PawnIterCaptureRight => {
                    self.state = PawnIterStates::PawnIterCaptureEnPassantLeft;
                    only_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(direction, 1)),
                        player,
                    )
                }
                PawnIterStates::PawnIterCaptureEnPassantLeft => {
                    self.state = PawnIterStates::PawnIterCaptureEnPassantRight;
                    only_en_passant(
                        &self.board_state.board,
                        &self.board_state.last_move,
                        try_move(&self.board_state.position, &dir!(direction, -1)),
                        player,
                        direction,
                    )
                }
                PawnIterStates::PawnIterCaptureEnPassantRight => {
                    self.state = PawnIterStates::PawnIterEnd;
                    only_en_passant(
                        &self.board_state.board,
                        &self.board_state.last_move,
                        try_move(&self.board_state.position, &dir!(direction, 1)),
                        player,
                        direction,
                    )
                }
                PawnIterStates::PawnIterEnd => return None,
            };

            match result {
                Some(position) => return Some(position),
                None => (),
            }
        }
    }
}

impl<'a> Iterator for KnightIter<'a> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        let square = self.board_state.board.square(self.board_state.position);
        let player = &square.unwrap().player;
        loop {
            let result = match self.state {
                KnightIterStates::KnightIter0 => {
                    self.state = KnightIterStates::KnightIter1;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(-1, -2)),
                        player,
                    )
                }
                KnightIterStates::KnightIter1 => {
                    self.state = KnightIterStates::KnightIter2;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(-1, 2)),
                        player,
                    )
                }
                KnightIterStates::KnightIter2 => {
                    self.state = KnightIterStates::KnightIter3;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(-2, -1)),
                        player,
                    )
                }
                KnightIterStates::KnightIter3 => {
                    self.state = KnightIterStates::KnightIter4;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(-2, 1)),
                        player,
                    )
                }
                KnightIterStates::KnightIter4 => {
                    self.state = KnightIterStates::KnightIter5;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(2, -1)),
                        player,
                    )
                }
                KnightIterStates::KnightIter5 => {
                    self.state = KnightIterStates::KnightIter6;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(2, 1)),
                        player,
                    )
                }
                KnightIterStates::KnightIter6 => {
                    self.state = KnightIterStates::KnightIter7;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(1, -2)),
                        player,
                    )
                }
                KnightIterStates::KnightIter7 => {
                    self.state = KnightIterStates::KnightIterEnd;
                    only_empty_or_enemy(
                        &self.board_state.board,
                        try_move(&self.board_state.position, &dir!(1, 2)),
                        player,
                    )
                }
                KnightIterStates::KnightIterEnd => return None,
            };

            match result {
                Some(position) => return Some(position),
                None => (),
            }
        }
    }
}

macro_rules! iter_path {
    ($self:ident, $dir:expr => $next_state:expr) => {{
        if $self.walker.is_none() {
            $self.walker = Some($self.walk($dir));
        }

        match $self
            .walker
            .as_mut()
            .unwrap()
            .next($self.board_state.board, &$self.player)
        {
            Some(position) => Some(position),
            None => {
                $self.state = $next_state;
                None
            }
        }
    }};
}

impl<'a> Iterator for BishopIter<'a> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let result = match self.state {
                BishopIterStates::BishopIter0 => {
                    iter_path!(self, dir!(-1, -1) => BishopIterStates::BishopIter1)
                }
                BishopIterStates::BishopIter1 => {
                    iter_path!(self, dir!(-1, 1) => BishopIterStates::BishopIter2)
                }
                BishopIterStates::BishopIter2 => {
                    iter_path!(self, dir!(1, -1) => BishopIterStates::BishopIter3)
                }
                BishopIterStates::BishopIter3 => {
                    iter_path!(self, dir!(1, 1) => BishopIterStates::BishopIterEnd)
                }
                BishopIterStates::BishopIterEnd => return None,
            };

            match result {
                Some(position) => return Some(position),
                None => (),
            }
        }
    }
}

/*
pub fn get_possible_moves<'a>(
    board: &Board,
    last_move: &Option<MoveInfo>,
    position: Position,
) -> impl Iterator<Item = &'a Position> {
    let square = &board.square(position);
    if square.is_none() {
        // println!("Square {} is empty", position);
        return std::iter::empty();
    }

    let piece = &square.unwrap().piece;
    let player = &square.unwrap().player;

    // println!("Square has a {} {}", player, piece);

    match piece {
        PieceType::Pawn => PawnIter {
            board,
            last_move,
            position,
            state: PawnIterStates::PawnIterNormal,
        },

        PieceType::Knight => collect_valid_moves([
            only_empty_or_enemy(&board, try_move(&position, &dir!(-1, -2)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(-1, 2)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(-2, -1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(-2, 1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(2, -1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(2, 1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(1, -2)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(1, 2)), player),
        ]),

        PieceType::Bishop => {
            let walker = WalkPath {
                board: &board,
                start: position,
                player: &player,
            };

            walker
                .walk(dir!(-1, -1))
                .chain(walker.walk(dir!(-1, 1)))
                .chain(walker.walk(dir!(1, -1)))
                .chain(walker.walk(dir!(1, 1)))
                .collect()
        }

        PieceType::Rook => {
            let walker = WalkPath {
                board: &board,
                start: position,
                player: &player,
            };

            walker
                .walk(dir!(0, -1))
                .chain(walker.walk(dir!(0, 1)))
                .chain(walker.walk(dir!(-1, 0)))
                .chain(walker.walk(dir!(1, 0)))
                .collect()
        }

        PieceType::Queen => {
            let walker = WalkPath {
                board: &board,
                start: position,
                player: &player,
            };

            walker
                .walk(dir!(-1, -1))
                .chain(walker.walk(dir!(-1, 1)))
                .chain(walker.walk(dir!(1, -1)))
                .chain(walker.walk(dir!(1, 1)))
                .chain(walker.walk(dir!(0, -1)))
                .chain(walker.walk(dir!(0, 1)))
                .chain(walker.walk(dir!(-1, 0)))
                .chain(walker.walk(dir!(1, 0)))
                .collect()
        }

        // TODO: castle
        PieceType::King => collect_valid_moves([
            only_empty_or_enemy(&board, try_move(&position, &dir!(-1, -1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(-1, 0)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(-1, 1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(0, -1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(0, 1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(1, -1)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(1, 0)), player),
            only_empty_or_enemy(&board, try_move(&position, &dir!(1, 1)), player),
        ]),
    }
}
*/
