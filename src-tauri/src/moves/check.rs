use crate::board::{Board, PieceType, Player, Position};
use crate::moves::conditions::{enemy, only_empty, try_move, Direction};
use crate::moves::iter::{dir, into_rolling_board_iterator, player_pieces_iter, PlayerPiecesIter};

pub fn find_player_king(board: &Board, player: &Player) -> Position {
    let king_pos_option = player_pieces_iter!(board: board, player: player)
        .into_iter()
        .filter(|pos| match board.square(pos) {
            Some(piece) => piece.piece == PieceType::King,
            None => false,
        })
        .collect::<Vec<Position>>();

    // No king? More than 1 king? o_O
    assert_eq!(
        king_pos_option.len(),
        1,
        "no king for player {}:\n{}",
        player,
        board
    );

    *king_pos_option.first().unwrap()
}

fn position_is_unsafe(board: &Board, position: &Position, player: &Player) -> bool {
    let enemy_player = enemy(&player);

    let is_player_piece = |position: &Option<Position>, piece: &PieceType| match position {
        Some(pos) => match board.square(pos) {
            Some(square) => square.piece == *piece && square.player == enemy_player,
            None => false,
        },
        None => false,
    };

    let enemy_in_direction = |direction: &Direction| {
        into_rolling_board_iterator(&board, &player, &position, direction)
            .find_map(|pos| board.square(&pos).as_ref())
            .map(|piece| piece.piece)
    };

    // 1. Pawns
    let pd = -Board::pawn_progress_direction(&enemy_player);

    if is_player_piece(&try_move(&position, &dir!(pd, -1)), &PieceType::Pawn)
        || is_player_piece(&try_move(&position, &dir!(pd, 1)), &PieceType::Pawn)
    {
        return true;
    }

    // 2. Knights
    if is_player_piece(&try_move(&position, &dir!(-1, -2)), &PieceType::Knight)
        || is_player_piece(&try_move(&position, &dir!(-1, 2)), &PieceType::Knight)
        || is_player_piece(&try_move(&position, &dir!(-2, -1)), &PieceType::Knight)
        || is_player_piece(&try_move(&position, &dir!(-2, 1)), &PieceType::Knight)
        || is_player_piece(&try_move(&position, &dir!(2, -1)), &PieceType::Knight)
        || is_player_piece(&try_move(&position, &dir!(2, 1)), &PieceType::Knight)
        || is_player_piece(&try_move(&position, &dir!(1, -2)), &PieceType::Knight)
        || is_player_piece(&try_move(&position, &dir!(1, 2)), &PieceType::Knight)
    {
        return true;
    }

    // 3. Bishops or queens on diagonals
    let bishop_or_queen = |direction: &Direction| match enemy_in_direction(direction) {
        Some(PieceType::Bishop) | Some(PieceType::Queen) => true,
        _ => false,
    };

    if bishop_or_queen(&dir!(-1, -1))
        || bishop_or_queen(&dir!(-1, 1))
        || bishop_or_queen(&dir!(1, -1))
        || bishop_or_queen(&dir!(1, 1))
    {
        return true;
    }

    // 4. Rooks or queens on files or ranks
    let rook_or_queen = |direction: &Direction| match enemy_in_direction(direction) {
        Some(PieceType::Rook) | Some(PieceType::Queen) => true,
        _ => false,
    };

    if rook_or_queen(&dir!(0, -1))
        || rook_or_queen(&dir!(0, 1))
        || rook_or_queen(&dir!(-1, 0))
        || rook_or_queen(&dir!(1, 0))
    {
        return true;
    }

    // 6. King
    if is_player_piece(&try_move(&position, &dir!(-1, -1)), &PieceType::King)
        || is_player_piece(&try_move(&position, &dir!(-1, 0)), &PieceType::King)
        || is_player_piece(&try_move(&position, &dir!(-1, 1)), &PieceType::King)
        || is_player_piece(&try_move(&position, &dir!(0, -1)), &PieceType::King)
        || is_player_piece(&try_move(&position, &dir!(0, 1)), &PieceType::King)
        || is_player_piece(&try_move(&position, &dir!(1, -1)), &PieceType::King)
        || is_player_piece(&try_move(&position, &dir!(1, 0)), &PieceType::King)
        || is_player_piece(&try_move(&position, &dir!(1, 1)), &PieceType::King)
    {
        return true;
    }

    false
}

pub fn piece_is_unsafe(board: &Board, position: &Position) -> bool {
    let square = board.square(position);
    let player = square.unwrap().player;
    position_is_unsafe(board, position, &player)
}

pub fn only_empty_and_safe(
    board: &Board,
    position: Option<Position>,
    player: &Player,
) -> Option<Position> {
    match &only_empty(board, position) {
        Some(position_value) => {
            if position_is_unsafe(board, position_value, player) {
                None
            } else {
                position
            }
        }
        None => None,
    }
}
