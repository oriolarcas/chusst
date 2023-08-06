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

pub struct WalkPath<'a> {
    board: &'a Board,
    start: Position,
    player: &'a Player,
}

impl<'a> WalkPath<'a> {
    pub fn walk(&'a self, direction: Direction) -> impl Iterator<Item = Position> + 'a {
        WalkPathIntoIter {
            path: self,
            position: self.start,
            direction,
            stop_walking: false,
        }
    }
}

struct WalkPathIntoIter<'a> {
    path: &'a WalkPath<'a>,
    position: Position,
    direction: Direction,
    stop_walking: bool,
}

impl<'a> Iterator for WalkPathIntoIter<'a> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop_walking {
            return None;
        }
        match only_empty_or_enemy(
            &self.path.board,
            try_move(&self.position, &self.direction),
            self.path.player,
        ) {
            Some(new_position) => {
                self.position = new_position;
                match only_empty(&self.path.board, Some(new_position)) {
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

enum PawnIterStates {
    PawnIterNormal,
    PawnIterPass,
    PawnIterCaptureLeft,
    PawnIterCaptureRight,
    PawnIterCaptureEnPassantLeft,
    PawnIterCaptureEnPassantRight,
    PawnIterEnd,
}

struct PawnIter<'a> {
    state: PawnIterStates,
    board_state: PieceIterBoardState<'a>,
}

enum KnightIterStates {
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

struct KnightIter<'a> {
    state: KnightIterStates,
    board_state: PieceIterBoardState<'a>,
}

enum PieceIterType<'a> {
    EmptySquareIterType(std::iter::Empty<Position>),
    PawnIterType(PawnIter<'a>),
    KnightIterType(KnightIter<'a>),
}

pub struct PieceIter<'a> {
    piece: PieceIterType<'a>,
}

pub fn piece_into_iter<'a>(
    board: &'a Board,
    last_move: &'a Option<MoveInfo>,
    position: Position,
) -> PieceIter<'a> {
    let square = &board.square(position);
    if square.is_none() {
        // println!("Square {} is empty", position);
        return PieceIter {
            piece: PieceIterType::EmptySquareIterType(std::iter::empty()),
        };
    }

    let piece = &square.unwrap().piece;

    let board_state = PieceIterBoardState {
        board,
        last_move,
        position,
    };

    match piece {
        PieceType::Pawn => PieceIter {
            piece: PieceIterType::PawnIterType(PawnIter {
                board_state,
                state: PawnIterStates::PawnIterNormal,
            }),
        },
        PieceType::Knight => PieceIter {
            piece: PieceIterType::KnightIterType(KnightIter {
                board_state,
                state: KnightIterStates::KnightIter0,
            }),
        },
        PieceType::Bishop => todo!(),
        PieceType::Rook => todo!(),
        PieceType::Queen => todo!(),
        PieceType::King => todo!(),
    }
}

impl<'a> Iterator for PieceIter<'a> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.piece {
            PieceIterType::EmptySquareIterType(iter) => iter.next(),
            PieceIterType::PawnIterType(iter) => iter.next(),
            PieceIterType::KnightIterType(iter) => iter.next(),
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
        let square = &self.board_state.board.square(self.board_state.position);
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
