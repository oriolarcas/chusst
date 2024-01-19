use crate::board::{Board, Direction, IterableBoard, Player, Position, PositionIterator};
use crate::game::{GameState, MoveExtraInfo, MoveInfo};

pub fn only_en_passant<B: Board>(
    game: &GameState<B>,
    position: Option<Position>,
    direction: i8,
    player: &Player,
    last_move: &Option<MoveInfo>,
) -> Option<Position> {
    match game.position_iter(&position?).only_empty().next() {
        Some(position_value) => {
            let reverse_direction = Direction {
                row_inc: -direction,
                col_inc: 0,
            };

            match (
                last_move,
                game.try_move(&position_value, &reverse_direction)
                    .only_enemy(*player)
                    .next(),
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
