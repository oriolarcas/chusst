mod check;
mod conditions;
mod iter;
mod play;

use crate::board::{Board, Game, Move, MoveInfo, Piece, PieceType, Player, Position, Rows};
use crate::moves::check::{find_player_king, player_in_check};
use crate::moves::play::SearchableGame;
use crate::{mv, pos};
use conditions::enemy;
use iter::{piece_into_iter, player_pieces_iter, BoardIter, PlayerPiecesIter};

use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

use self::iter::pawn_progress_direction;
use self::play::{PlayableGame, ReversableGame};

// List of pieces that can capture each square
pub type BoardCaptures = Rows<Vec<Position>>;

type Score = i32;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum MateType {
    Checkmate,
    Stalemate,
}

pub enum GameMove {
    Normal(Move),
    Mate(MateType),
}

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

fn get_possible_moves_iter<'a>(
    board: &'a Board,
    last_move: &'a Option<MoveInfo>,
    position: Position,
) -> impl Iterator<Item = Position> + 'a {
    piece_into_iter(board, last_move, position)
}

pub fn get_possible_moves_no_checks(
    board: &Board,
    last_move: &Option<MoveInfo>,
    position: Position,
) -> Vec<Position> {
    get_possible_moves_iter(board, last_move, position).collect::<Vec<Position>>()
}

pub fn get_possible_moves(
    board: &Board,
    last_move: &Option<MoveInfo>,
    position: Position,
) -> Vec<Position> {
    let square = board.square(&position).as_ref();
    if square.is_none() {
        return vec![];
    }

    let player = square.unwrap().player;
    let is_king = square.unwrap().piece == PieceType::King;

    let king_position = if is_king {
        position
    } else {
        find_player_king(&board, &player)
    };

    let mut game = Game {
        board: *board,
        player,
        last_move: None,
    };
    get_possible_moves_no_checks(board, last_move, position)
        .iter()
        .filter(|possible_position| {
            let mv = mv!(position, **possible_position);

            let mut rev_game = SearchableGame::from_game(&mut game);

            assert!(
                rev_game.do_move_no_checks(&mv),
                "Unexpected invalid move {} in:\n{}",
                mv,
                &rev_game.as_ref().board,
            );

            let current_king_position = if is_king {
                *possible_position
            } else {
                &king_position
            };

            let is_check = player_in_check(&rev_game.as_ref().board, &current_king_position);

            !is_check
        })
        .copied()
        .collect()
}

pub fn move_name(
    board: &Board,
    last_move: &Option<MoveInfo>,
    player: &Player,
    mv: &Move,
) -> Option<String> {
    let mut name = String::new();
    let src_piece_opt = board.square(&mv.source);
    if src_piece_opt.is_none() {
        return None;
    }
    let src_piece = src_piece_opt.unwrap();
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

    // Is check?
    let mut game = Game {
        board: *board,
        player: src_piece.player,
        last_move: *last_move,
    };

    if do_move(&mut game, mv).is_some() {
        let enemy_king_position = find_player_king(&game.board, &enemy(player));
        let causes_check = get_possible_captures_of_position(&game.board, last_move, &mv.target)
            .iter()
            .find(|position| **position == enemy_king_position)
            .is_some();
        if causes_check {
            let is_checkmate = get_best_move_recursive(&mut game, 0).is_none();

            name.push(if is_checkmate { '#' } else { '+' });
        }
    }

    Some(name)
}

pub fn move_branch_names(board: &Board, player: &Player, moves: &Vec<&Move>) -> Vec<String> {
    let mut game = Game {
        board: *board,
        player: *player,
        last_move: None,
    };

    let mut move_names = vec![];

    for mv in moves {
        move_names.push(move_name(&game.board, &game.last_move, &game.player, &mv).unwrap());

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

    let player = game.player;
    let king_position = find_player_king(&game.board, &game.player);

    for player_piece_position in pieces_iter {
        let current_piece = &game.board.square(&player_piece_position).unwrap().piece;

        for possible_position in
            get_possible_moves_no_checks(&game.board, &game.last_move, player_piece_position)
        {
            searched_moves += 1;

            let local_score = match &game.board.square(&possible_position) {
                Some(piece) => get_piece_value(piece.piece),
                None => {
                    if *current_piece == PieceType::Pawn
                        && possible_position.row == Board::promotion_rank(&player)
                    {
                        // Promotion
                        get_piece_value(PieceType::Queen)
                    } else {
                        0
                    }
                }
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

            let mut rev_game = SearchableGame::from_game(game);

            assert!(
                rev_game.do_move_no_checks(&mv.mv),
                "Unexpected invalid move {} in:\n{}",
                mv.mv,
                &rev_game.as_ref().board,
            );

            let current_king_position = if *current_piece == PieceType::King {
                possible_position
            } else {
                king_position
            };

            // Check if this move is invalid because it leaves the king in check
            if player_in_check(&rev_game.as_ref().board, &current_king_position) {
                continue;
            }

            // Recursion
            if search_depth > 0 {
                let mut next_moves_opt =
                    get_best_move_recursive(rev_game.as_mut(), search_depth - 1);

                let is_check_mate = if next_moves_opt.is_none() {
                    // check or stale mate?
                    let enemy_player_king_position =
                        find_player_king(&rev_game.as_ref().board, &enemy(&player));

                    player_in_check(&rev_game.as_ref().board, &enemy_player_king_position)
                } else {
                    false
                };

                let is_stale_mate = next_moves_opt.is_none() && !is_check_mate;

                if is_check_mate {
                    branch.score = local_score.saturating_add(get_piece_value(PieceType::King));
                } else if !is_stale_mate {
                    let next_moves = next_moves_opt.as_mut().unwrap();

                    branch.moves.append(&mut next_moves.moves);
                    branch.score = local_score.saturating_sub(next_moves.score);
                    branch.searched = next_moves.searched;
                }

                searched_moves += branch.searched;
            };

            match &best_move {
                Some(current_best_move) => {
                    if branch.score > current_best_move.score
                        || (branch.score == current_best_move.score
                            && branch.moves.len() < current_best_move.moves.len())
                    {
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

fn get_possible_captures_of_position(
    board: &Board,
    last_move: &Option<MoveInfo>,
    position: &Position,
) -> Vec<Position> {
    let mut captures: Vec<Position> = Vec::new();

    match board.square(position) {
        Some(square) => {
            for possible_position in get_possible_moves_iter(&board, last_move, *position) {
                let is_capture = board.square(&possible_position).is_some();

                if is_capture {
                    captures.push(possible_position);
                } else if !is_capture
                    && square.piece == PieceType::Pawn
                    && position.col.abs_diff(position.col) != 0
                {
                    let passed_rank = usize::try_from(
                        i8::try_from(position.row).unwrap()
                            - pawn_progress_direction(&square.player),
                    )
                    .unwrap();
                    captures.push(pos!(passed_rank, possible_position.col));
                }
            }
        }
        None => (),
    }

    captures
}

pub fn get_possible_captures(board: &Board, last_move: &Option<MoveInfo>) -> BoardCaptures {
    let board_iter: BoardIter = Default::default();
    let mut board_captures: BoardCaptures = Default::default();

    for source_position in board_iter.into_iter() {
        for capture in get_possible_captures_of_position(board, last_move, &source_position) {
            board_captures[capture.row][capture.col].push(source_position);
        }
    }

    board_captures
}

pub fn get_best_move(game: &mut Game, search_depth: u32) -> GameMove {
    let player = game.player;
    let start_time = Instant::now();
    let best_branch = get_best_move_recursive(game, search_depth);
    let duration = (Instant::now() - start_time).as_secs_f64();

    if best_branch.is_none() {
        // check or stale mate?
        let king_position = find_player_king(&game.board, &player);
        let is_check_mate = player_in_check(&game.board, &king_position);

        print!("  ({:.2} s.) ", duration);
        let enemy_player = enemy(&player);
        if is_check_mate {
            println!("Checkmate, {} wins", enemy_player);
        } else {
            println!("Stalemate caused by {}", enemy_player);
        }
        return if is_check_mate {
            GameMove::Mate(MateType::Checkmate)
        } else {
            GameMove::Mate(MateType::Stalemate)
        };
    }

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

    GameMove::Normal(**branch_moves.first().unwrap())
}

pub fn is_mate(board: &Board, player: &Player, last_move: &Option<MoveInfo>) -> Option<MateType> {
    // Is check?
    let mut game = Game {
        board: *board,
        player: *player,
        last_move: *last_move,
    };

    if get_best_move_recursive(&mut game, 0).is_none() {
        let king_position = find_player_king(&board, &player);
        return if player_in_check(&board, &king_position) {
            Some(MateType::Checkmate)
        } else {
            Some(MateType::Stalemate)
        };
    }

    None
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

    let mut rev_game = <ReversableGame as PlayableGame>::from_game(game);
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
    use crate::moves::play::ReversableGame;
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
        board: Option<Vec<PiecePosition>>,
        initial_moves: Vec<Move>,
        mv: Move,
        checks: Vec<PiecePosition>,
    }

    fn custom_board(initial_pieces_opt: &Option<Vec<PiecePosition>>) -> Board {
        match initial_pieces_opt {
            Some(initial_pieces) => {
                let mut board: Board = Default::default();

                for piece_position in initial_pieces {
                    board.update(&piece_position.position, piece_position.piece);
                }

                board
            }
            None => *initial_board(),
        }
    }

    #[test]
    fn move_reversable() {
        let test_boards = [
            // Advance pawn
            TestBoard {
                board: None,
                initial_moves: vec![],
                mv: mv!(e2 => e3),
                checks: vec![pp!(pw @ e3), pp!(e2)],
            },
            // Pass pawn
            TestBoard {
                board: None,
                initial_moves: vec![],
                mv: mv!(e2 => e4),
                checks: vec![pp!(pw @ e4), pp!(e2)],
            },
            // Pawn capturing
            TestBoard {
                board: None,
                initial_moves: vec![mv!(e2 => e4), mv!(d7 => d5)],
                mv: mv!(e4 => d5),
                checks: vec![pp!(pw @ d5), pp!(e4)],
            },
            // Pawn capturing en passant
            TestBoard {
                board: None,
                initial_moves: vec![mv!(e2 => e4), mv!(a7 => a6), mv!(e4 => e5), mv!(d7 => d5)],
                mv: mv!(e5 => d6),
                checks: vec![pp!(pw @ d6), pp!(e5), pp!(d5)],
            },
            // Pawn promotion
            TestBoard {
                board: None,
                initial_moves: vec![
                    mv!(h2 => h4),
                    mv!(g7 => g6),
                    mv!(h4 => h5),
                    mv!(a7 => a6),
                    mv!(h5 => g6),
                    mv!(a6 => a5),
                    mv!(g6 => g7),
                    mv!(a5 => a4),
                ],
                mv: mv!(g7 => h8),
                checks: vec![pp!(qw @ h8)],
            },
        ];

        for test_board in &test_boards {
            // Prepare board
            let mut game = Game {
                board: custom_board(&test_board.board),
                player: Player::White,
                last_move: None,
            };

            // Do setup moves
            for mv in &test_board.initial_moves {
                assert!(
                    do_move(&mut game, &mv).is_some(),
                    "move {} failed:\n{}",
                    mv,
                    game.board
                );
            }

            let original_board = game.board.clone();

            let mut rev_game = <ReversableGame as PlayableGame>::from_game(&mut game);

            // Do move
            assert!(rev_game.do_move(&test_board.mv));

            for check in &test_board.checks {
                assert_eq!(
                    *rev_game.as_ref().board.square(&check.position),
                    check.piece,
                    "expected {} in {}, found {}:\n{}",
                    check
                        .piece
                        .map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                    check.position,
                    rev_game
                        .as_ref()
                        .board
                        .square(&check.position)
                        .map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                    rev_game.as_ref().board,
                );
            }

            rev_game.undo();

            assert_eq!(
                game.board, original_board,
                "after move {},\nmodified board:\n{}\noriginal board:\n{}",
                test_board.mv, game.board, original_board
            );
        }
    }

    #[test]
    fn check_mate() {
        let test_boards = [
            TestBoard {
                board: Some(vec![pp!(kw @ a1), pp!(qb @ b3), pp!(qb @ c2), pp!(kb @ h8)]),
                initial_moves: vec![],
                mv: mv!(b3 => b2),
                checks: vec![],
            },
            TestBoard {
                board: Some(vec![
                    pp!(kw @ a1),
                    pp!(pb @ b3),
                    pp!(pb @ c2),
                    pp!(qb @ a3),
                    pp!(kb @ h8),
                ]),
                initial_moves: vec![],
                mv: mv!(b3 => b2),
                checks: vec![],
            },
            TestBoard {
                board: Some(vec![pp!(kw @ a1), pp!(rb @ b8), pp!(rb @ b7), pp!(kb @ h8)]),
                initial_moves: vec![],
                mv: mv!(b8 => a8),
                checks: vec![],
            },
            TestBoard {
                board: Some(vec![
                    pp!(kw @ a1),
                    pp!(bb @ f8),
                    pp!(bb @ g8),
                    pp!(pb @ c2),
                    pp!(kb @ h8),
                ]),
                initial_moves: vec![],
                mv: mv!(f8 => g7),
                checks: vec![],
            },
            TestBoard {
                board: Some(vec![
                    pp!(kw @ a1),
                    pp!(nb @ a5),
                    pp!(nb @ c3),
                    pp!(rb @ h2),
                    pp!(kb @ h8),
                ]),
                initial_moves: vec![],
                mv: mv!(a5 => b3),
                checks: vec![],
            },
        ];

        for test_board in test_boards {
            // Prepare board
            let mut game = Game {
                board: custom_board(&test_board.board),
                player: Player::Black,
                last_move: None,
            };

            // Do setup moves
            for mv in &test_board.initial_moves {
                assert!(
                    do_move(&mut game, &mv).is_some(),
                    "move {} failed:\n{}",
                    mv,
                    game.board
                );
            }

            let name =
                move_name(&game.board, &game.last_move, &game.player, &test_board.mv).unwrap();
            assert!(
                name.ends_with("#"),
                "notation {} doesn't show checkmate sign #",
                name
            );

            // Do move
            let mut rev_game = <ReversableGame as PlayableGame>::from_game(&mut game);

            assert!(
                rev_game.do_move(&test_board.mv),
                "invalid move {}:\n{}",
                test_board.mv,
                rev_game.as_ref().board
            );

            let possible_moves = get_possible_moves(&game.board, &None, pos!(a1));
            let in_check = player_in_check(&game.board, &pos!(a1));
            assert!(in_check, "king should be in check:\n{}", game.board);
            assert!(
                possible_moves.is_empty(),
                "unexpected possible move {} in check mate:\n{}",
                mv!(pos!(a1), *possible_moves.first().unwrap()),
                game.board
            );
        }
    }
}
