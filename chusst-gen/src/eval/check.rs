#[cfg(feature = "compact-board")]
use crate::board::CompactBoard;
#[cfg(feature = "bitboards")]
use crate::board::{Bitboards, ModifiableBoard, PlayerBitboards};
use crate::board::{
    Board, Direction, Piece, PieceType, Player, Position, PositionIterator, SimpleBoard,
};
use crate::eval::iter::dir;

fn find_king(board: &impl Board, player: &Player) -> Position {
    match board.iter().find(|position| {
        Some(Piece {
            piece: PieceType::King,
            player: *player,
        }) == board.at(position)
    }) {
        Some(position) => position,
        None => panic!("no king for player {}:\n{}", player, board),
    }
}

fn is_position_unsafe(
    board: &impl Board,
    position: &Position,
    player: &Player,
    enemy_pawn_direction: i8,
) -> bool {
    let is_enemy_piece = |direction: Direction, piece: PieceType| {
        board
            .try_move(position, &direction)
            .only_enemy_piece(*player, piece)
            .next()
            .is_some()
    };

    let enemy_in_direction = |direction: &Direction| {
        board
            .direction_iterator(position, direction)
            .first_non_empty()
            .only_enemy(*player)
            .next()
            .and_then(|pos| board.at(&pos))
            .map(|piece| piece.piece)
    };

    // 1. Pawns
    if is_enemy_piece(dir!(enemy_pawn_direction, -1), PieceType::Pawn)
        || is_enemy_piece(dir!(enemy_pawn_direction, 1), PieceType::Pawn)
    {
        return true;
    }

    // 2. Knights
    if is_enemy_piece(dir!(-1, -2), PieceType::Knight)
        || is_enemy_piece(dir!(-1, 2), PieceType::Knight)
        || is_enemy_piece(dir!(-2, -1), PieceType::Knight)
        || is_enemy_piece(dir!(-2, 1), PieceType::Knight)
        || is_enemy_piece(dir!(2, -1), PieceType::Knight)
        || is_enemy_piece(dir!(2, 1), PieceType::Knight)
        || is_enemy_piece(dir!(1, -2), PieceType::Knight)
        || is_enemy_piece(dir!(1, 2), PieceType::Knight)
    {
        return true;
    }

    // 3. Bishops or queens on diagonals
    let bishop_or_queen = |direction: &Direction| {
        matches!(
            enemy_in_direction(direction),
            Some(PieceType::Bishop) | Some(PieceType::Queen)
        )
    };

    if bishop_or_queen(&dir!(-1, -1))
        || bishop_or_queen(&dir!(-1, 1))
        || bishop_or_queen(&dir!(1, -1))
        || bishop_or_queen(&dir!(1, 1))
    {
        return true;
    }

    // 4. Rooks or queens on files or ranks
    let rook_or_queen = |direction: &Direction| {
        matches!(
            enemy_in_direction(direction),
            Some(PieceType::Rook) | Some(PieceType::Queen)
        )
    };

    if rook_or_queen(&dir!(0, -1))
        || rook_or_queen(&dir!(0, 1))
        || rook_or_queen(&dir!(-1, 0))
        || rook_or_queen(&dir!(1, 0))
    {
        return true;
    }

    // 6. King
    if is_enemy_piece(dir!(-1, -1), PieceType::King)
        || is_enemy_piece(dir!(-1, 0), PieceType::King)
        || is_enemy_piece(dir!(-1, 1), PieceType::King)
        || is_enemy_piece(dir!(0, -1), PieceType::King)
        || is_enemy_piece(dir!(0, 1), PieceType::King)
        || is_enemy_piece(dir!(1, -1), PieceType::King)
        || is_enemy_piece(dir!(1, 0), PieceType::King)
        || is_enemy_piece(dir!(1, 1), PieceType::King)
    {
        return true;
    }

    false
}

fn is_position_unsafe_generic<B: Board>(board: &B, position: &Position, player: &Player) -> bool {
    let pd = -B::pawn_progress_direction(&!*player);
    is_position_unsafe(board, position, player, pd)
}

fn is_piece_unsafe(board: &impl Board, position: &Position) -> bool {
    let Some(Piece { piece: _, player }) = board.at(position) else {
        panic!("No piece at position {}:\n{}", position, board);
    };

    is_position_unsafe_generic(board, position, &player)
}

pub fn only_empty_and_safe<B: Board>(
    board: &B,
    position: Option<Position>,
    player: &Player,
) -> Option<Position> {
    let position_value = board.position_iter(&position?).only_empty().next()?;
    if is_position_unsafe_generic(board, &position_value, player) {
        None
    } else {
        position
    }
}

pub trait SafetyChecks {
    fn find_king(&self, player: &Player) -> Position;
    fn is_position_unsafe(&self, position: &Position, player: &Player) -> bool;
    fn is_piece_unsafe(&self, position: &Position) -> bool;
}

impl SafetyChecks for SimpleBoard {
    fn find_king(&self, player: &Player) -> Position {
        find_king(self, player)
    }

    fn is_position_unsafe(&self, position: &Position, player: &Player) -> bool {
        is_position_unsafe_generic(self, position, player)
    }

    fn is_piece_unsafe(&self, position: &Position) -> bool {
        is_piece_unsafe(self, position)
    }
}

#[cfg(feature = "compact-board")]
impl SafetyChecks for CompactBoard {
    fn find_king(&self, player: &Player) -> Position {
        find_king(self, player)
    }

    fn is_position_unsafe(&self, position: &Position, player: &Player) -> bool {
        is_position_unsafe_generic(self, position, player)
    }

    fn is_piece_unsafe(&self, position: &Position) -> bool {
        is_piece_unsafe(self, position)
    }
}

#[cfg(feature = "bitboards")]
impl SafetyChecks for Bitboards {
    fn find_king(&self, player: &Player) -> Position {
        let player_bitboards = self.by_player(player);
        let king_position = player_bitboards.piece_iter(&PieceType::King).next();
        assert!(
            king_position.is_some(),
            "Player has no king:\n{}",
            player_bitboards
        );
        king_position.expect("Player has no king!")
    }

    fn is_position_unsafe(&self, position: &Position, player: &Player) -> bool {
        let player_bitboards = self.by_player(player);
        let enemy_bitboards = self.by_player(&!*player);

        let all_pieces_bitboard = player_bitboards.combined() | enemy_bitboards.combined();

        let attacker_in_rank =
            |attacker_position: &Position| attacker_position.rank == position.rank;

        let attacker_in_file =
            |attacker_position: &Position| attacker_position.file == position.file;

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
                    if in_between_mask & all_pieces_bitboard == 0 {
                        return true;
                    }
                }
            }
            false
        };

        let enemy_in_diagonal = |piece: &PieceType| {
            let attacker_bitboard = enemy_bitboards.by_piece(piece);
            for attacker_position in PlayerBitboards::into_iter(attacker_bitboard) {
                if attacker_in_diagonal(&Some(attacker_position)) {
                    let in_between_mask = PlayerBitboards::in_between(&attacker_position, position);
                    if in_between_mask & all_pieces_bitboard == 0 {
                        return true;
                    }
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
                    if in_between_mask & all_pieces_bitboard == 0 {
                        return true;
                    }
                }
            }
            false
        };

        // 1. Pawns
        if enemy_bitboards.pawn_can_attack(position) {
            return true;
        }

        // 2. Knights
        if enemy_bitboards.knight_can_attack(position) {
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
        if enemy_bitboards.king_can_attack(position) {
            return true;
        }

        false
    }

    fn is_piece_unsafe(&self, position: &Position) -> bool {
        let Some(Piece { piece: _, player }) = self.at(position) else {
            panic!("No piece at position {}:\n{}", position, self);
        };
        self.is_position_unsafe(position, &player)
    }
}
