use crate::board::{Board, Game, Move, PieceType, Player, Position};

struct Direction {
    row_inc: i8,
    col_inc: i8,
}

macro_rules! dir {
    ($row:expr, $col:expr) => {
        Direction {
            row_inc: $row,
            col_inc: $col,
        }
    };
}

struct WalkPath<'a> {
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

struct BoardIter {
    position: Position,
}

impl Default for BoardIter {
    fn default() -> Self {
        BoardIter {
            position: Position { row: 0, col: 0 },
        }
    }
}

impl<'a> Iterator for BoardIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position.row > 7 {
            return None;
        }

        let current_position = self.position;

        if self.position.col == 7 {
            self.position.row += 1;
            self.position.col = 0;
        } else {
            self.position.col += 1;
        }

        Some(current_position)
    }
}

struct PlayerPiecesIter<'a> {
    board: &'a Board,
    player: &'a Player,
    board_iter: BoardIter,
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

impl<'a> Iterator for PlayerPiecesIter<'a> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.board_iter.next() {
                Some(position) => match self.board.rows[position.row][position.col] {
                    Some(piece) => {
                        if piece.player == *self.player {
                            return Some(position);
                        }
                        continue;
                    }
                    None => {
                        continue;
                    }
                },
                None => {
                    return None;
                }
            }
        }
    }
}

fn try_move(position: &Position, direction: &Direction) -> Option<Position> {
    let row = i8::try_from(position.row).unwrap() + direction.row_inc;
    let col = i8::try_from(position.col).unwrap() + direction.col_inc;

    if row < 0 || row >= 8 {
        return None;
    }

    if col < 0 || col >= 8 {
        return None;
    }

    match (usize::try_from(row), usize::try_from(col)) {
        (Ok(urow), Ok(ucol)) => Some(Position {
            row: urow,
            col: ucol,
        }),
        _ => None,
    }
}

fn only_empty(board: &Board, position: Option<Position>) -> Option<Position> {
    match position {
        Some(position_value) => match board.rows[position_value.row][position_value.col] {
            Some(_) => None,
            None => Some(position_value),
        },
        None => None,
    }
}

fn only_player(board: &Board, position: Option<Position>, player: Player) -> Option<Position> {
    match position {
        Some(position_value) => match board.rows[position_value.row][position_value.col] {
            Some(square) => {
                if square.player == player {
                    Some(position_value)
                } else {
                    None
                }
            }
            None => None,
        },
        None => None,
    }
}

fn enemy(player: &Player) -> Player {
    match player {
        Player::White => Player::Black,
        Player::Black => Player::White,
    }
}

fn only_enemy(board: &Board, position: Option<Position>, player: &Player) -> Option<Position> {
    only_player(board, position, enemy(player))
}

fn only_empty_or_enemy(
    board: &Board,
    position: Option<Position>,
    player: &Player,
) -> Option<Position> {
    match position {
        Some(position_value) => match board.rows[position_value.row][position_value.col] {
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

fn collect_valid_moves<const N: usize>(positions: [Option<Position>; N]) -> Vec<Position> {
    positions
        .iter()
        .filter(|position| position.is_some())
        .map(|position| position.as_ref().unwrap())
        .copied()
        .collect()
}

pub fn get_possible_moves(board: &Board, position: Position) -> Vec<Position> {
    let square = &board.rows[position.row][position.col];
    if square.is_none() {
        // println!("Square {} is empty", position);
        return vec![];
    }

    let piece = &square.unwrap().piece;
    let player = &square.unwrap().player;

    // println!("Square has a {} {}", player, piece);

    match piece {
        PieceType::Pawn => {
            let direction = match player {
                Player::White => 1,
                Player::Black => -1,
            };
            let can_pass = match player {
                Player::White => position.row == 1,
                Player::Black => position.row == 6,
            };
            let normal = only_empty(&board, try_move(&position, &dir!(direction, 0)));
            let passed = if can_pass && normal.is_some() {
                only_empty(&board, try_move(&position, &dir!(direction * 2, 0)))
            } else {
                None
            };
            let captures = (
                only_enemy(&board, try_move(&position, &dir!(direction, -1)), player),
                only_enemy(&board, try_move(&position, &dir!(direction, 1)), player),
            );
            // TODO: capture passed pawns

            collect_valid_moves([normal, passed, captures.0, captures.1])
        }

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

pub fn move_name(game: &Game, mv: &Move) -> String {
    let mut name = String::new();
    let src_piece = game.board.rows[mv.source.row][mv.source.col].unwrap();
    let tgt_piece_opt = game.board.rows[mv.target.row][mv.target.col];

    let pieces_iter = player_pieces_iter!(board: &game.board, player: &game.turn);

    match src_piece.piece {
        PieceType::Pawn => {}
        PieceType::Knight => name.push('N'),
        PieceType::Bishop => name.push('B'),
        PieceType::Rook => name.push('R'),
        PieceType::Queen => name.push('Q'),
        PieceType::King => name.push('K'),
    }

    let mut piece_in_same_file = false;
    let mut piece_in_same_rank = false;

    for player_piece_position in pieces_iter {
        if player_piece_position == mv.source {
            continue;
        }

        match game.board.rows[player_piece_position.row][player_piece_position.col] {
            Some(player_piece) => {
                if player_piece.piece != src_piece.piece {
                    continue;
                }
            }
            None => {}
        }

        if get_possible_moves(&game.board, player_piece_position)
            .iter()
            .find(|possible_position| **possible_position == mv.target)
            .is_some()
        {
            if player_piece_position.row == mv.source.row {
                piece_in_same_rank = true;
            } else if player_piece_position.col == mv.source.col {
                piece_in_same_file = true;
            }
        }
    }

    let source_suffix = format!("{}", mv.source);
    if piece_in_same_file && piece_in_same_rank {
        // Same type of pieces in same rank and file: file and rank suffix
        name.push_str(source_suffix.as_str());
    } else if piece_in_same_rank {
        // Same type of pieces in same rank but different file: file suffix
        name.push(source_suffix.chars().nth(0).unwrap());
    } else if piece_in_same_file {
        // Same type of pieces in same file but different rank: rank suffix
        name.push(source_suffix.chars().nth(1).unwrap());
    }

    if tgt_piece_opt.is_some() {
        name.push('x');
    }

    name.push_str(format!("{}", mv.target).as_str());

    name
}

// pub fn get_best_move(game: &Game) -> Move {

// }

pub fn do_move(game: &mut Game, mv: Move) -> bool {
    let board = &mut game.board;
    let possible_moves = get_possible_moves(&board, mv.source);

    if possible_moves
        .iter()
        .find(|possible_position| mv.target == **possible_position)
        .is_none()
    {
        return false;
    }

    board.rows[mv.target.row][mv.target.col] = board.rows[mv.source.row][mv.source.col];
    board.rows[mv.source.row][mv.source.col] = None;

    game.turn = enemy(&game.turn);

    return true;
}
