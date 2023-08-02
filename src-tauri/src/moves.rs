use std::time::Instant;

use crate::board::{Board, Game, Move, MoveExtraInfo, MoveInfo, PieceType, Player, Position, Rows};

// List of pieces that can capture each square
pub type BoardCaptures = Rows<Vec<Position>>;

type Score = i32;

#[derive(PartialEq)]
struct WeightedMove {
    pub mv: Move,
    pub score: Score,
}

#[derive(Default)]
struct Branch {
    pub moves: Vec<WeightedMove>,
    pub score: Score,
    pub searched: u32,
}

impl PartialEq for Branch {
    fn eq(&self, other: &Self) -> bool {
        match (self.moves.first(), other.moves.first()) {
            (None, None) => true,
            (Some(self_move), Some(other_move)) => self_move == other_move,
            (_, _) => false,
        }
    }
}

impl PartialOrd for Branch {
    fn partial_cmp(&self, other: &Self) -> std::option::Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

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

fn collect_valid_moves<const N: usize>(positions: [Option<Position>; N]) -> Vec<Position> {
    positions
        .iter()
        .filter(|position| position.is_some())
        .map(|position| position.as_ref().unwrap())
        .copied()
        .collect()
}

pub fn get_possible_moves(
    board: &Board,
    last_move: &Option<MoveInfo>,
    position: Position,
) -> Vec<Position> {
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

            collect_valid_moves([
                normal,
                if can_pass && normal.is_some() {
                    only_empty(&board, try_move(&position, &dir!(direction * 2, 0)))
                } else {
                    None
                },
                // Normal captures
                only_enemy(&board, try_move(&position, &dir!(direction, -1)), player),
                only_enemy(&board, try_move(&position, &dir!(direction, 1)), player),
                // En passant captures
                only_en_passant(
                    &board,
                    &last_move,
                    try_move(&position, &dir!(direction, -1)),
                    player,
                    direction,
                ),
                only_en_passant(
                    &board,
                    &last_move,
                    try_move(&position, &dir!(direction, 1)),
                    player,
                    direction,
                ),
            ])
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

pub fn move_name(
    board: &Board,
    last_move: &Option<MoveInfo>,
    player: &Player,
    mv: &Move,
) -> String {
    let mut name = String::new();
    let src_piece = board.rows[mv.source.row][mv.source.col].unwrap();
    let tgt_piece_opt = board.rows[mv.target.row][mv.target.col];

    let pieces_iter = player_pieces_iter!(board: board, player: player);

    match src_piece.piece {
        PieceType::Pawn => {}
        PieceType::Knight => name.push('N'),
        PieceType::Bishop => name.push('B'),
        PieceType::Rook => name.push('R'),
        PieceType::Queen => name.push('Q'),
        PieceType::King => name.push('K'),
    }

    let is_en_passant = src_piece.piece == PieceType::Pawn
        && mv.source.col != mv.target.col
        && board.rows[mv.target.row][mv.target.col].is_none();

    let mut piece_in_same_file = false;
    let mut piece_in_same_rank = false;

    for player_piece_position in pieces_iter {
        if player_piece_position == mv.source {
            continue;
        }

        match board.rows[player_piece_position.row][player_piece_position.col] {
            Some(player_piece) => {
                if player_piece.piece != src_piece.piece {
                    continue;
                }
            }
            None => {}
        }

        if get_possible_moves(board, last_move, player_piece_position)
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
    if is_en_passant {
        name.push(source_suffix.chars().nth(0).unwrap());
    } else if piece_in_same_file && piece_in_same_rank {
        // Same type of pieces in same rank and file: file and rank suffix
        name.push_str(source_suffix.as_str());
    } else if piece_in_same_rank {
        // Same type of pieces in same rank but different file: file suffix
        name.push(source_suffix.chars().nth(0).unwrap());
    } else if piece_in_same_file {
        // Same type of pieces in same file but different rank: rank suffix
        name.push(source_suffix.chars().nth(1).unwrap());
    }

    if tgt_piece_opt.is_some() || is_en_passant {
        name.push('x');
    }

    name.push_str(format!("{}", mv.target).as_str());

    name
}

pub fn move_branch_names(board: &Board, player: &Player, moves: &Vec<&Move>) -> Vec<String> {
    let mut game = Game {
        board: *board,
        player: *player,
        turn: 0,
        last_move: None,
    };

    let mut move_names = vec![];

    for mv in moves {
        move_names.push(move_name(&game.board, &game.last_move, &game.player, &mv));

        assert!(do_move(&mut game, *mv));
    }

    move_names
}

fn get_piece_value(piece: PieceType) -> Score {
    match piece {
        PieceType::Pawn => 100,
        PieceType::Knight => 300,
        PieceType::Bishop => 300,
        PieceType::Rook => 500,
        PieceType::Queen => 900,
        PieceType::King => Score::MAX,
    }
}

fn get_best_move_recursive(
    board: &Board,
    last_move: &Option<MoveInfo>,
    player: &Player,
    search_depth: u32,
) -> Option<Branch> {
    let pieces_iter = player_pieces_iter!(board: board, player: player);

    let mut best_move: Option<Branch> = None;

    let mut searched_moves: u32 = 0;

    for player_piece_position in pieces_iter {
        for possible_position in get_possible_moves(&board, last_move, player_piece_position) {
            searched_moves += 1;

            let local_score = match board.rows[possible_position.row][possible_position.col] {
                Some(piece) => get_piece_value(piece.piece),
                None => 0,
            };

            let mut branch = Branch {
                moves: vec![WeightedMove {
                    mv: Move {
                        source: player_piece_position,
                        target: possible_position,
                    },
                    score: local_score,
                }],
                score: local_score,
                searched: 0,
            };

            let mv = branch.moves.first().unwrap();

            // Recursion
            if search_depth > 0 {
                let mut game = Game {
                    board: *board,
                    player: *player,
                    turn: 0,
                    last_move: *last_move,
                };

                assert!(
                    do_move(&mut game, &mv.mv),
                    "Unexpected invalid move {}",
                    mv.mv
                );

                let mut next_moves = get_best_move_recursive(
                    &game.board,
                    &game.last_move,
                    &game.player,
                    search_depth - 1,
                )
                .unwrap();

                branch.moves.append(&mut next_moves.moves);
                branch.score = local_score.saturating_sub(next_moves.score);
                branch.searched = next_moves.searched;

                searched_moves += branch.searched;
            };

            match &mut best_move {
                Some(current_best_move) => {
                    if current_best_move < &mut branch {
                        best_move = Some(branch);
                    }
                }
                None => {
                    best_move = Some(branch);
                }
            }
        }
    }

    match &mut best_move {
        Some(best_move) => {
            best_move.searched = searched_moves;
        }
        None => (),
    }

    best_move
}

pub fn get_possible_captures(board: &Board, last_move: &Option<MoveInfo>) -> BoardCaptures {
    let board_iter: BoardIter = Default::default();
    let mut board_captures: BoardCaptures = Default::default();

    for source_position in board_iter.into_iter() {
        match board.rows[source_position.row][source_position.col] {
            Some(square) => {
                for possible_position in get_possible_moves(&board, last_move, source_position) {
                    let is_capture =
                        board.rows[possible_position.row][possible_position.col].is_some();

                    if is_capture {
                        board_captures[possible_position.row][possible_position.col]
                            .push(source_position);
                    } else if !is_capture && square.piece == PieceType::Pawn {
                        if possible_position.col.abs_diff(source_position.col) != 0 {}
                    }
                }
            }
            None => (),
        }
    }

    board_captures
}

pub fn get_best_move(
    board: &Board,
    last_move: &Option<MoveInfo>,
    player: &Player,
) -> Option<Vec<Move>> {
    let start_time = Instant::now();
    let best_branch = get_best_move_recursive(board, last_move, player, 3);
    let duration = (Instant::now() - start_time).as_secs_f64();

    let total_moves = best_branch
        .as_ref()
        .map(|weighted_move| weighted_move.searched)
        .unwrap();

    let branch_moves = best_branch
        .as_ref()
        .unwrap()
        .moves
        .iter()
        .map(|mv| &mv.mv)
        .collect::<Vec<&Move>>();

    println!(
        "  ({:.2} s., {:.0} mps) Best branch after {}: {}",
        duration,
        f64::from(total_moves) / duration,
        total_moves,
        std::iter::zip(
            &best_branch.as_ref().unwrap().moves,
            move_branch_names(&board, &player, &branch_moves)
        )
        .map(|(move_info, move_name)| format!("{}{:+}", move_name, move_info.score))
        .collect::<Vec<String>>()
        .join(" "),
    );

    Some(branch_moves.iter().map(|move_ref| **move_ref).collect())
}

pub fn do_move(game: &mut Game, mv: &Move) -> bool {
    let board = &mut game.board;

    match &board.rows[mv.source.row][mv.source.col] {
        Some(piece) => {
            if piece.player != game.player {
                return false;
            }
        }
        None => {
            return false;
        }
    }

    let possible_moves = get_possible_moves(&board, &game.last_move, mv.source);

    if possible_moves
        .iter()
        .find(|possible_position| mv.target == **possible_position)
        .is_none()
    {
        return false;
    }

    let player = board.rows[mv.source.row][mv.source.col].unwrap().player;
    let moved_piece = board.rows[mv.source.row][mv.source.col].unwrap().piece;
    let move_info = match moved_piece {
        PieceType::Pawn => {
            if mv.source.row.abs_diff(mv.target.row) == 2 {
                MoveExtraInfo::Passed
            } else if mv.source.col != mv.target.col
                && board.rows[mv.target.row][mv.target.col].is_none()
            {
                MoveExtraInfo::EnPassant
            } else {
                MoveExtraInfo::Other
            }
        }
        _ => MoveExtraInfo::Other,
    };

    board.rows[mv.target.row][mv.target.col] = board.rows[mv.source.row][mv.source.col];
    board.rows[mv.source.row][mv.source.col] = None;

    match move_info {
        MoveExtraInfo::EnPassant => {
            // Capture passed pawn
            let direction: i8 = match player {
                Player::White => 1,
                Player::Black => -1,
            };
            let passed =
                only_enemy(&board, try_move(&mv.target, &dir!(-direction, 0)), &player).unwrap();
            board.rows[passed.row][passed.col] = None;
        }
        _ => (),
    }

    game.player = enemy(&game.player);
    game.last_move = Some(MoveInfo {
        mv: *mv,
        info: move_info,
    });

    return true;
}
