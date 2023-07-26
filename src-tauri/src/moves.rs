use crate::board::{Board, PieceType, Player, Position};

fn try_move(position: &Position, row_inc: i8, col_inc: i8) -> Option<Position> {
    let row = i8::try_from(position.row).unwrap() + row_inc;
    let col = i8::try_from(position.col).unwrap() + col_inc;

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

fn only_enemy(board: &Board, position: Option<Position>, player: &Player) -> Option<Position> {
    only_player(
        board,
        position,
        match player {
            Player::White => Player::Black,
            Player::Black => Player::White,
        },
    )
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
        println!("Square {} is empty", position);
        return vec![];
    }

    let piece = &square.unwrap().piece;
    let player = &square.unwrap().player;

    println!("Square has a {} {}", player, piece);

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
            let normal = only_empty(&board, try_move(&position, direction, 0));
            let passed = if can_pass /* TODO: && not blocked */ {
                only_empty(&board, try_move(&position, direction * 2, 0))
            } else {
                None
            };
            let captures = (
                only_enemy(&board, try_move(&position, direction, -1), player),
                only_enemy(&board, try_move(&position, direction, 1), player),
            );
            // TODO: capture passed pawns

            collect_valid_moves([normal, passed, captures.0, captures.1])
        }
        PieceType::Knight => collect_valid_moves([
            only_empty_or_enemy(&board, try_move(&position, -1, -2), player),
            only_empty_or_enemy(&board, try_move(&position, -1, 2), player),
            only_empty_or_enemy(&board, try_move(&position, -2, -1), player),
            only_empty_or_enemy(&board, try_move(&position, -2, 1), player),
            only_empty_or_enemy(&board, try_move(&position, 2, -1), player),
            only_empty_or_enemy(&board, try_move(&position, 2, 1), player),
            only_empty_or_enemy(&board, try_move(&position, 1, -2), player),
            only_empty_or_enemy(&board, try_move(&position, 1, 2), player),
        ]),
        PieceType::Bishop => {
            vec![]
        }
        PieceType::Rook => {
            vec![]
        }
        PieceType::Queen => {
            vec![]
        }
        PieceType::King => collect_valid_moves([
            only_empty_or_enemy(&board, try_move(&position, -1, -1), player),
            only_empty_or_enemy(&board, try_move(&position, -1, 0), player),
            only_empty_or_enemy(&board, try_move(&position, -1, 1), player),
            only_empty_or_enemy(&board, try_move(&position, 0, -1), player),
            only_empty_or_enemy(&board, try_move(&position, 0, 1), player),
            only_empty_or_enemy(&board, try_move(&position, 1, -1), player),
            only_empty_or_enemy(&board, try_move(&position, 1, 0), player),
            only_empty_or_enemy(&board, try_move(&position, 1, 1), player),
        ]),
    }
}
