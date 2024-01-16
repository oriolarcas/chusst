use crate::board::{Board, Player, Position};
use crate::game::{MoveExtraInfo, MoveInfo};
use crate::pos;

#[derive(Copy, Clone)]
pub struct Direction {
    pub row_inc: i8,
    pub col_inc: i8,
}

pub fn try_move(position: &Position, direction: &Direction) -> Option<Position> {
    let row = i8::try_from(position.rank).unwrap() + direction.row_inc;
    let col = i8::try_from(position.file).unwrap() + direction.col_inc;

    if !(0..8).contains(&row) {
        return None;
    }

    if !(0..8).contains(&col) {
        return None;
    }

    match (usize::try_from(row), usize::try_from(col)) {
        (Ok(urow), Ok(ucol)) => Some(pos!(urow, ucol)),
        _ => None,
    }
}

pub fn only_empty(board: &impl Board, position: Option<Position>) -> Option<Position> {
    match board.at(&position?) {
        Some(_) => None,
        None => position,
    }
}

fn only_player(board: &impl Board, position: Option<Position>, player: Player) -> Option<Position> {
    match board.at(&position?) {
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

pub fn only_enemy(
    board: &impl Board,
    position: Option<Position>,
    player: &Player,
) -> Option<Position> {
    only_player(board, position, !*player)
}

pub fn only_empty_or_enemy(
    board: &impl Board,
    position: Option<Position>,
    player: &Player,
) -> Option<Position> {
    match position {
        Some(position_value) => match board.at(&position_value) {
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

pub fn only_en_passant(
    board: &impl Board,
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
