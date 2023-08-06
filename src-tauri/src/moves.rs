use std::time::Instant;

use crate::board::{
    Board, Game, Move, MoveExtraInfo, MoveInfo, Piece, PieceType, Player, Position, Rows,
};

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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
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
            position: pos!(0, 0),
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
                Some(position) => match self.board.square(position) {
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

struct ReversableMove {
    mv: Move,
    previous_piece: Option<PieceType>,
}

struct ReversableGame<'a> {
    game: &'a mut Game,
    moves: Vec<ReversableMove>,
    last_move: Option<MoveInfo>,
    move_player: Player,
}

impl<'a> ReversableGame<'a> {
    fn from(game: &mut Game) -> ReversableGame {
        let player = game.player;
        ReversableGame {
            game,
            moves: vec![],
            last_move: None,
            move_player: player,
        }
    }

    fn do_move(&mut self, mv: &Move) -> bool {
        assert!(self.moves.is_empty());

        let board = &mut self.game.board;

        match board.square(mv.source) {
            Some(piece) => {
                if piece.player != self.game.player {
                    return false;
                }
            }
            None => {
                return false;
            }
        }

        let possible_moves = get_possible_moves(&board, &self.game.last_move, mv.source);

        if possible_moves
            .iter()
            .find(|possible_position| mv.target == **possible_position)
            .is_none()
        {
            return false;
        }

        let player = board.square(mv.source).unwrap().player;
        let moved_piece = board.square(mv.source).unwrap().piece;
        let move_info = match moved_piece {
            PieceType::Pawn => {
                if mv.source.row.abs_diff(mv.target.row) == 2 {
                    MoveExtraInfo::Passed
                } else if mv.source.col != mv.target.col && board.square(mv.target).is_none() {
                    MoveExtraInfo::EnPassant
                } else {
                    MoveExtraInfo::Other
                }
            }
            _ => MoveExtraInfo::Other,
        };

        self.moves.push(ReversableMove {
            mv: *mv,
            previous_piece: board.square(mv.target).map(|piece| piece.piece),
        });

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
                    only_enemy(&board, try_move(&mv.target, &dir!(-direction, 0)), &player)
                        .unwrap();

                self.moves.push(ReversableMove {
                    mv: mv!(passed, passed),
                    previous_piece: board.square(passed).map(|piece| piece.piece),
                });

                board.rows[passed.row][passed.col] = None;
            }
            _ => (),
        }

        self.game.player = enemy(&self.game.player);
        self.last_move = self.game.last_move;
        self.game.last_move = Some(MoveInfo {
            mv: *mv,
            info: move_info,
        });

        return true;
    }

    fn undo(&mut self) {
        assert!(!self.moves.is_empty());

        for rev_move in self.moves.iter().rev() {
            let mv = &rev_move.mv;

            self.game.board.rows[mv.source.row][mv.source.col] =
                self.game.board.rows[mv.target.row][mv.target.col];

            match rev_move.previous_piece {
                Some(piece) => {
                    self.game.board.rows[mv.target.row][mv.target.col] = Some(Piece {
                        piece,
                        player: enemy(&self.move_player),
                    })
                }
                None => self.game.board.rows[mv.target.row][mv.target.col] = None,
            }
        }

        self.moves.clear();
        self.game.player = enemy(&self.game.player);
        self.game.last_move = self.last_move;
        self.last_move = None;
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
    let square = &board.square(position);
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
    let src_piece = board.square(mv.source).unwrap();
    let tgt_piece_opt = board.square(mv.target);

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
        && board.square(mv.target).is_none();

    let mut piece_in_same_file = false;
    let mut piece_in_same_rank = false;

    for player_piece_position in pieces_iter {
        if player_piece_position == mv.source {
            continue;
        }

        match board.square(player_piece_position) {
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

fn get_best_move_recursive(game: &mut Game, search_depth: u32) -> Option<Branch> {
    let pieces_iter =
        player_pieces_iter!(board: &game.board, player: &game.player).collect::<Vec<Position>>();

    let mut best_move: Option<Branch> = None;

    let mut searched_moves: u32 = 0;

    for player_piece_position in pieces_iter {
        for possible_position in
            get_possible_moves(&game.board, &game.last_move, player_piece_position)
        {
            searched_moves += 1;

            let local_score = match &game.board.square(possible_position) {
                Some(piece) => get_piece_value(piece.piece),
                None => 0,
            };

            let mut branch = Branch {
                moves: vec![WeightedMove {
                    mv: mv!(player_piece_position, possible_position),
                    score: local_score,
                }],
                score: local_score,
                searched: 0,
            };

            let mv = branch.moves.first().unwrap();

            // Recursion
            if search_depth > 0 {
                let mut rev_game = ReversableGame::from(game);

                assert!(
                    rev_game.do_move(&mv.mv),
                    "Unexpected invalid move {}",
                    mv.mv
                );

                let mut next_moves =
                    get_best_move_recursive(&mut rev_game.game, search_depth - 1).unwrap();

                rev_game.undo();

                branch.moves.append(&mut next_moves.moves);
                branch.score = local_score.saturating_sub(next_moves.score);
                branch.searched = next_moves.searched;

                searched_moves += branch.searched;
            };

            match &best_move {
                Some(current_best_move) => {
                    if &branch > current_best_move {
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
        match board.square(source_position) {
            Some(square) => {
                for possible_position in get_possible_moves(&board, last_move, source_position) {
                    let is_capture = board.square(possible_position).is_some();

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

pub fn get_best_move(game: &mut Game, search_depth: u32) -> Option<Vec<Move>> {
    let start_time = Instant::now();
    let best_branch = get_best_move_recursive(game, search_depth);
    let duration = (Instant::now() - start_time).as_secs_f64();

    let total_score = best_branch
        .as_ref()
        .map(|weighted_move| weighted_move.score)
        .unwrap();

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
        "  ({:.2} s., {:.0} mps) Best branch {:+} after {}: {}",
        duration,
        f64::from(total_moves) / duration,
        total_score,
        total_moves,
        std::iter::zip(
            &best_branch.as_ref().unwrap().moves,
            move_branch_names(&game.board, &game.player, &branch_moves)
        )
        .map(|(move_info, move_name)| format!("{}{:+}", move_name, move_info.score))
        .collect::<Vec<String>>()
        .join(" "),
    );

    Some(branch_moves.iter().map(|move_ref| **move_ref).collect())
}

pub fn do_move(game: &mut Game, mv: &Move) -> bool {
    let rev_game = &mut ReversableGame::from(game);
    rev_game.do_move(mv)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PiecePosition {
        piece: Option<Piece>,
        position: Position,
    }

    macro_rules! pp {
        ($piece:ident @ $pos:ident) => {
            PiecePosition {
                piece: p!($piece),
                position: pos!($pos),
            }
        };
        ($pos:ident) => {
            PiecePosition {
                piece: None,
                position: pos!($pos),
            }
        };
    }

    struct TestBoard {
        initial_moves: Vec<Move>,
        mv: Move,
        checks: Vec<PiecePosition>,
    }

    #[test]
    fn move_reversable() {
        let test_boards = [
            // Advance pawn
            TestBoard {
                initial_moves: vec![],
                mv: mv!(e2 => e3),
                checks: vec![pp!(pw @ e3), pp!(e2)],
            },
            // Pass pawn
            TestBoard {
                initial_moves: vec![],
                mv: mv!(e2 => e4),
                checks: vec![pp!(pw @ e4), pp!(e2)],
            },
            // Pawn capturing
            TestBoard {
                initial_moves: vec![mv!(e2 => e4), mv!(d7 => d5)],
                mv: mv!(e4 => d5),
                checks: vec![pp!(pw @ d5), pp!(e4)],
            },
            // Pawn capturing en passant
            TestBoard {
                initial_moves: vec![mv!(e2 => e4), mv!(a7 => a6), mv!(e4 => e5), mv!(d7 => d5)],
                mv: mv!(e5 => d6),
                checks: vec![pp!(pw @ d6), pp!(e5), pp!(d5)],
            },
        ];

        for test_board in &test_boards {
            // Prepare board
            let mut game = Game {
                board: initial_board!(),
                player: Player::White,
                last_move: None,
            };

            // Do setup moves
            for mv in &test_board.initial_moves {
                assert!(do_move(&mut game, &mv));
            }

            let original_board = game.board.clone();

            let mut rev_game = ReversableGame::from(&mut game);

            // Do move
            assert!(rev_game.do_move(&test_board.mv));

            for check in &test_board.checks {
                assert_eq!(*rev_game.game.board.square(check.position), check.piece);
            }

            rev_game.undo();

            assert_eq!(rev_game.game.board, original_board);
        }
    }
}
