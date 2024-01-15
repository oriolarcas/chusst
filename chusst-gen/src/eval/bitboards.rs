use crate::board::{Board, ModifiableBoard, Piece, PieceType, Player, Position};
use crate::game::Game;

use std::fmt;

type Bitboard = u64;

impl From<&Game> for BitboardGame {
    fn from(game: &Game) -> BitboardGame {
        let mut bitboards = BitboardGame {
            game: game.clone(),
            white: PlayerBitboards::new(Player::White),
            black: PlayerBitboards::new(Player::Black),
        };

        for rank in 0..8 {
            for file in 0..8 {
                let Some(square) = game.board.square(&Position { rank, file }) else {
                    continue;
                };
                let player_bitboards = match square.player {
                    Player::White => &mut bitboards.white,
                    Player::Black => &mut bitboards.black,
                };

                let piece_mask = bitboard_from_rank_and_file(rank, file);

                match square.piece {
                    PieceType::Pawn => player_bitboards.pawns |= piece_mask,
                    PieceType::Knight => player_bitboards.knights |= piece_mask,
                    PieceType::Bishop => player_bitboards.bishops |= piece_mask,
                    PieceType::Rook => player_bitboards.rooks |= piece_mask,
                    PieceType::Queen => player_bitboards.queens |= piece_mask,
                    PieceType::King => player_bitboards.kings |= piece_mask,
                }

                player_bitboards.combined |= piece_mask;
            }
        }

        bitboards
    }
}
