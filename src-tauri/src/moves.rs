mod check;
mod conditions;
mod iter;

use crate::board::{
    Board, Game, Move, MoveExtraInfo, MoveInfo, Piece, PieceType, Player, Position, Rows,
};
use crate::moves::check::{player_in_check, find_player_king};
use crate::mv;
use conditions::{enemy, only_enemy, try_move, Direction};
use iter::{dir, piece_into_iter, player_pieces_iter, BoardIter, PlayerPiecesIter};

use std::collections::HashMap;
use std::time::Instant;

// List of pieces that can capture each square
pub type BoardCaptures = Rows<Vec<Position>>;

type Score = i32;

#[derive(PartialEq)]
pub struct WeightedMove {
    pub mv: Move,
    pub score: Score,
}

#[derive(Default)]
pub struct Branch {
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

struct ReversableMove {
    mv: Move,
    previous_piece: Option<PieceType>,
}

trait SearchableGame<'a> {
    fn from_game(game: &'a mut Game) -> Self
    where
        Self: Sized;
    fn as_ref(&self) -> &Game;
    fn as_mut(&mut self) -> &mut Game;
    fn do_move(&mut self, mv: &Move) -> bool;
    fn do_move_no_checks(&mut self, mv: &Move) -> bool;
    fn undo(&mut self);
}

struct ReversableGame<'a> {
    game: &'a mut Game,
    moves: Vec<ReversableMove>,
    last_move: Option<MoveInfo>,
    move_player: Player,
}

impl<'a> SearchableGame<'a> for ReversableGame<'a> {
    fn from_game(game: &'a mut Game) -> Self {
        let player = game.player;
        ReversableGame {
            game,
            moves: vec![],
            last_move: None,
            move_player: player,
        }
    }

    fn as_ref(&self) -> &Game {
        &self.game
    }

    fn as_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    fn do_move_no_checks(&mut self, mv: &Move) -> bool {
        let board = &mut self.game.board;
        let player = board.square(&mv.source).unwrap().player;
        let moved_piece = board.square(&mv.source).unwrap().piece;
        let move_info = match moved_piece {
            PieceType::Pawn => {
                if mv.source.row.abs_diff(mv.target.row) == 2 {
                    MoveExtraInfo::Passed
                } else if mv.source.col != mv.target.col && board.square(&mv.target).is_none() {
                    MoveExtraInfo::EnPassant
                } else {
                    MoveExtraInfo::Other
                }
            }
            _ => MoveExtraInfo::Other,
        };

        self.moves.push(ReversableMove {
            mv: *mv,
            previous_piece: board.square(&mv.target).map(|piece| piece.piece),
        });

        board.move_piece(&mv.source, &mv.target);

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
                    previous_piece: board.square(&passed).map(|piece| piece.piece),
                });

                board.update(&passed, None);
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

    fn do_move(&mut self, mv: &Move) -> bool {
        assert!(self.moves.is_empty());

        let board = &self.game.board;

        match board.square(&mv.source) {
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

        self.do_move_no_checks(mv)
    }

    fn undo(&mut self) {
        assert!(!self.moves.is_empty());

        for rev_move in self.moves.iter().rev() {
            let mv = &rev_move.mv;

            self.game.board.move_piece(&mv.target, &mv.source);

            match rev_move.previous_piece {
                Some(piece) => self.game.board.update(
                    &mv.target,
                    Some(Piece {
                        piece,
                        player: enemy(&self.move_player),
                    }),
                ),
                None => self.game.board.update(&mv.target, None),
            }
        }

        self.moves.clear();
        self.game.player = enemy(&self.game.player);
        self.game.last_move = self.last_move;
        self.last_move = None;
    }
}

struct ClonedGame(Game);

impl<'a> SearchableGame<'a> for ClonedGame {
    fn from_game(game: &mut Game) -> Self {
        ClonedGame(*game)
    }

    fn as_ref(&self) -> &Game {
        &self.0
    }

    fn as_mut(&mut self) -> &mut Game {
        &mut self.0
    }

    fn do_move_no_checks(&mut self, mv: &Move) -> bool {
        let board = &mut self.0.board;
        let player = board.square(&mv.source).unwrap().player;
        let moved_piece = board.square(&mv.source).unwrap().piece;
        let move_info = match moved_piece {
            PieceType::Pawn => {
                if mv.source.row.abs_diff(mv.target.row) == 2 {
                    MoveExtraInfo::Passed
                } else if mv.source.col != mv.target.col && board.square(&mv.target).is_none() {
                    MoveExtraInfo::EnPassant
                } else {
                    MoveExtraInfo::Other
                }
            }
            _ => MoveExtraInfo::Other,
        };

        board.move_piece(&mv.source, &mv.target);

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

                board.update(&passed, None);
            }
            _ => (),
        }

        self.0.player = enemy(&self.0.player);
        self.0.last_move = Some(MoveInfo {
            mv: *mv,
            info: move_info,
        });

        return true;
    }

    fn do_move(&mut self, mv: &Move) -> bool {
        let board = &self.0.board;

        match board.square(&mv.source) {
            Some(piece) => {
                if piece.player != self.0.player {
                    return false;
                }
            }
            None => {
                return false;
            }
        }

        let possible_moves = get_possible_moves(&board, &self.0.last_move, mv.source);

        if possible_moves
            .iter()
            .find(|possible_position| mv.target == **possible_position)
            .is_none()
        {
            return false;
        }

        self.do_move_no_checks(mv)
    }

    fn undo(&mut self) {}
}

fn get_possible_moves_iter<'a>(
    board: &'a Board,
    last_move: &'a Option<MoveInfo>,
    position: Position,
) -> impl Iterator<Item = Position> + 'a {
    piece_into_iter(board, last_move, position)
}

pub fn get_possible_moves(
    board: &Board,
    last_move: &Option<MoveInfo>,
    position: Position,
) -> Vec<Position> {
    get_possible_moves_iter(board, last_move, position).collect::<Vec<Position>>()
}

pub fn move_name(
    board: &Board,
    last_move: &Option<MoveInfo>,
    player: &Player,
    mv: &Move,
) -> String {
    let mut name = String::new();
    let src_piece = board.square(&mv.source).unwrap();
    let tgt_piece_opt = board.square(&mv.target);

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
        && board.square(&mv.target).is_none();

    let mut piece_in_same_file = false;
    let mut piece_in_same_rank = false;

    for player_piece_position in pieces_iter {
        if player_piece_position == mv.source {
            continue;
        }

        match board.square(&player_piece_position) {
            Some(player_piece) => {
                if player_piece.piece != src_piece.piece {
                    continue;
                }
            }
            None => {}
        }

        if get_possible_moves_iter(board, last_move, player_piece_position)
            .find(|possible_position| *possible_position == mv.target)
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

        assert!(do_move(&mut game, *mv).is_some());
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

pub fn get_best_move_recursive(game: &mut Game, search_depth: u32) -> Option<Branch> {
    let pieces_iter =
        player_pieces_iter!(board: &game.board, player: &game.player).collect::<Vec<Position>>();

    let mut best_move: Option<Branch> = None;

    let mut searched_moves: u32 = 0;

    let king_position = find_player_king(&game.board, &game.player);

    for player_piece_position in pieces_iter {
        let current_piece = &game.board.square(&player_piece_position).unwrap().piece;

        for possible_position in
            get_possible_moves(&game.board, &game.last_move, player_piece_position)
        {
            searched_moves += 1;

            let local_score = match &game.board.square(&possible_position) {
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

            let mut rev_game: Box<dyn SearchableGame> = {
                #[cfg(feature = "search-by-rollback")]
                {
                    Box::new(ReversableGame::from_game(game))
                }
                #[cfg(not(feature = "search-by-rollback"))]
                {
                    Box::new(ClonedGame::from_game(game))
                }
            };

            assert!(
                rev_game.do_move_no_checks(&mv.mv),
                "Unexpected invalid move {} in:\n{}",
                mv.mv,
                &rev_game.as_ref().as_ref().board,
            );

            let current_king_position = if *current_piece == PieceType::King {
                possible_position
            } else {
                king_position
            };

            // Check if this move is invalid because it leaves the king in check
            if player_in_check(&rev_game.as_ref().as_ref().board, &current_king_position) {
                continue;
            }

            // Recursion
            if search_depth > 0 {
                let mut next_moves =
                    get_best_move_recursive(rev_game.as_mut().as_mut(), search_depth - 1).unwrap();

                branch.moves.append(&mut next_moves.moves);
                branch.score = local_score.saturating_sub(next_moves.score);
                branch.searched = next_moves.searched;

                searched_moves += branch.searched;
            };

            rev_game.undo();

            drop(rev_game);

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
        match board.square(&source_position) {
            Some(square) => {
                for possible_position in get_possible_moves_iter(&board, last_move, source_position)
                {
                    let is_capture = board.square(&possible_position).is_some();

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

pub fn do_move(game: &mut Game, mv: &Move) -> Option<Vec<Piece>> {
    let enemy_player = enemy(&game.player);
    let mut enemy_army: HashMap<PieceType, u32> = HashMap::new();

    for piece in player_pieces_iter!(board: &game.board, player: &enemy_player)
        .map(|position| game.board.square(&position).unwrap().piece)
    {
        match enemy_army.get_mut(&piece) {
            Some(piece_count) => {
                *piece_count += 1;
            }
            None => {
                enemy_army.insert(piece, 1);
            }
        }
    }

    let rev_game = &mut ReversableGame::from_game(game);
    let result = rev_game.do_move(mv);

    if result {
        for piece in player_pieces_iter!(board: &game.board, player: &enemy_player)
            .map(|position| game.board.square(&position).unwrap().piece)
        {
            match enemy_army.get_mut(&piece) {
                Some(piece_count) => {
                    *piece_count -= 1;
                    if *piece_count == 0 {
                        assert!(enemy_army.remove(&piece).is_some());
                    }
                }
                None => {
                    // A pawn has been promoted to something else (promotion is mandatory)
                    assert_ne!(piece, PieceType::Pawn);
                    assert!(enemy_army.remove(&PieceType::Pawn).is_some());
                }
            }
        }
        Some(
            enemy_army
                .keys()
                .into_iter()
                .map(|piece| Piece {
                    piece: *piece,
                    player: enemy_player,
                })
                .collect(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::initial_board;
    use crate::{p, pos};

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
                board: *initial_board(),
                player: Player::White,
                last_move: None,
            };

            // Do setup moves
            for mv in &test_board.initial_moves {
                assert!(do_move(&mut game, &mv).is_some());
            }

            let original_board = game.board.clone();

            let mut rev_game = ReversableGame::from_game(&mut game);

            // Do move
            assert!(rev_game.do_move(&test_board.mv));

            for check in &test_board.checks {
                assert_eq!(*rev_game.game.board.square(&check.position), check.piece);
            }

            rev_game.undo();

            assert_eq!(
                rev_game.game.board, original_board,
                "after move {},\nmodified board:\n{}\noriginal board:\n{}",
                test_board.mv, rev_game.game.board, original_board
            );
        }
    }
}
