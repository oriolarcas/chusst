use crate::board::{Board, PieceType, Player, Position};
use crate::eval::bitboards::PlayerBitboards;
use crate::eval::conditions::{enemy, only_empty, try_move, Direction};
use crate::eval::iter::{dir, into_rolling_board_iterator, player_pieces_iter, PlayerPiecesIter};

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

pub fn find_player_king_fast(
    player_bitboards: &PlayerBitboards,
) -> Position {
    let king_position = player_bitboards
        .piece_iter(&PieceType::King)
        .next();
    assert!(king_position.is_some(), "Player has no king:\n{}", player_bitboards);
    king_position.expect("Player has no king!")
}

fn position_is_unsafe_by_squares(board: &Board, position: &Position, player: &Player) -> bool {
    let enemy_player = enemy(&player);

    let is_enemy_piece = |position: &Option<Position>, piece: &PieceType| match position {
        Some(pos) => match board.square(pos) {
            Some(square) => square.piece == *piece && square.player == enemy_player,
            None => false,
        },
        None => false,
    };

    let enemy_in_direction = |direction: &Direction| {
        into_rolling_board_iterator(&board, &player, &position, direction)
            .find_map(|pos| board.square(&pos))
            .map(|piece| piece.piece)
    };

    // 1. Pawns
    let pd = -Board::pawn_progress_direction(&enemy_player);

    if is_enemy_piece(&try_move(&position, &dir!(pd, -1)), &PieceType::Pawn)
        || is_enemy_piece(&try_move(&position, &dir!(pd, 1)), &PieceType::Pawn)
    {
        return true;
    }

    // 2. Knights
    if is_enemy_piece(&try_move(&position, &dir!(-1, -2)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(-1, 2)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(-2, -1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(-2, 1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(2, -1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(2, 1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(1, -2)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(1, 2)), &PieceType::Knight)
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
    if is_enemy_piece(&try_move(&position, &dir!(-1, -1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(-1, 0)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(-1, 1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(0, -1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(0, 1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(1, -1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(1, 0)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(1, 1)), &PieceType::King)
    {
        return true;
    }

    false
}

fn position_is_unsafe_by_bitboards(
    player_bitboards: &PlayerBitboards,
    enemy_bitboards: &PlayerBitboards,
    position: &Position,
    player: &Player,
) -> bool {
    let enemy_player = enemy(&player);
    let all_pieces_bitboard = player_bitboards.combined() | enemy_bitboards.combined();

    let is_enemy_piece = |position: &Option<Position>, piece: &PieceType| match position {
        Some(pos) => enemy_bitboards.has_piece(pos, piece),
        None => false,
    };

    let attacker_in_rank = |attacker_position: &Position| attacker_position.rank == position.rank;

    let attacker_in_file = |attacker_position: &Position| attacker_position.file == position.file;

    let attacker_in_diagonal = |attacker_position: &Option<Position>| match attacker_position {
        Some(pos) => {
            let rank_diff = (pos.rank as i8 - position.rank as i8).abs();
            let file_diff = (pos.file as i8 - position.file as i8).abs();
            rank_diff == file_diff
        }
        None => false,
    };

    let enemy_in_rank_or_file = |piece: &PieceType| {
        let attacker_bitboard = enemy_bitboards.by_piece(piece);
        for attacker_position in PlayerBitboards::into_iter(attacker_bitboard) {
            if attacker_in_rank(&attacker_position) || attacker_in_file(&attacker_position) {
                let in_between_mask = PlayerBitboards::in_between(&attacker_position, position);
                return in_between_mask & all_pieces_bitboard == 0;
            }
        }
        false
    };

    let enemy_in_diagonal = |piece: &PieceType| {
        let attacker_bitboard = enemy_bitboards.by_piece(piece);
        for attacker_position in PlayerBitboards::into_iter(attacker_bitboard) {
            if attacker_in_diagonal(&Some(attacker_position)) {
                let in_between_mask = PlayerBitboards::in_between(&attacker_position, position);
                return in_between_mask & all_pieces_bitboard == 0;
            }
        }
        false
    };

    let enemy_in_rank_file_or_diagonal = |piece: &PieceType| {
        let attacker_bitboard = enemy_bitboards.by_piece(piece);
        for attacker_position in PlayerBitboards::into_iter(attacker_bitboard) {
            if attacker_in_rank(&attacker_position)
                || attacker_in_file(&attacker_position)
                || attacker_in_diagonal(&Some(attacker_position))
            {
                let in_between_mask = PlayerBitboards::in_between(&attacker_position, position);
                return in_between_mask & all_pieces_bitboard == 0;
            }
        }
        false
    };

    // 1. Pawns
    let pd = -Board::pawn_progress_direction(&enemy_player);

    if is_enemy_piece(&try_move(&position, &dir!(pd, -1)), &PieceType::Pawn)
        || is_enemy_piece(&try_move(&position, &dir!(pd, 1)), &PieceType::Pawn)
    {
        return true;
    }

    // 2. Knights
    if is_enemy_piece(&try_move(&position, &dir!(-1, -2)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(-1, 2)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(-2, -1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(-2, 1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(2, -1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(2, 1)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(1, -2)), &PieceType::Knight)
        || is_enemy_piece(&try_move(&position, &dir!(1, 2)), &PieceType::Knight)
    {
        return true;
    }

    // 3. Bishops
    if enemy_in_diagonal(&PieceType::Bishop) {
        return true;
    }

    // 4. Rooks
    if enemy_in_rank_or_file(&PieceType::Rook) {
        return true;
    }

    // 5. Queens
    if enemy_in_rank_file_or_diagonal(&PieceType::Queen) {
        return true;
    }

    // 6. King
    if is_enemy_piece(&try_move(&position, &dir!(-1, -1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(-1, 0)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(-1, 1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(0, -1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(0, 1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(1, -1)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(1, 0)), &PieceType::King)
        || is_enemy_piece(&try_move(&position, &dir!(1, 1)), &PieceType::King)
    {
        return true;
    }

    false
}

pub fn piece_is_unsafe(board: &Board, position: &Position) -> bool {
    let square = board.square(position);
    let player = square.unwrap().player;
    position_is_unsafe_by_squares(board, position, &player)
}

pub fn piece_is_unsafe_fast(
    board: &Board,
    position: &Position,
    player_bitboards: &PlayerBitboards,
    enemy_bitboards: &PlayerBitboards,
) -> bool {
    let square = board.square(position);
    let player = square.unwrap().player;
    position_is_unsafe_by_bitboards(player_bitboards, enemy_bitboards, position, &player)
}

pub fn only_empty_and_safe(
    board: &Board,
    position: Option<Position>,
    player: &Player,
    player_bitboards: &PlayerBitboards,
    enemy_bitboards: &PlayerBitboards,
) -> Option<Position> {
    match &only_empty(board, position) {
        Some(position_value) => {
            if position_is_unsafe_by_bitboards(
                player_bitboards,
                enemy_bitboards,
                position_value,
                player,
            ) {
                None
            } else {
                position
            }
        }
        None => None,
    }
}
