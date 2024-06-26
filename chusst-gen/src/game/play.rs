use super::{
    CastlingRights, GameInfo, GameState, MoveAction, MoveActionType, MoveExtraInfo, MoveInfo,
};
use crate::board::{
    Board, Direction, IterableBoard, ModifiableBoard, Piece, PieceType, Player, Position,
    PositionIterator,
};
use crate::dir;
use anyhow::{bail, Result};

pub trait ModifiableGame<B: Board>:
    ModifiableBoard<Position, Option<Piece>> + CastlingRights + IterableBoard
{
    fn board(&self) -> &B;
    fn board_mut(&mut self) -> &mut B;

    fn player(&self) -> Player;
    fn update_player(&mut self, player: Player);

    fn info(&self) -> &GameInfo;

    fn last_move(&self) -> &Option<MoveInfo>;

    fn do_move_no_checks(&mut self, mv: &MoveAction) -> Result<()>;
}

impl<B: Board> ModifiableGame<B> for GameState<B> {
    fn board(&self) -> &B {
        &self.board
    }

    fn board_mut(&mut self) -> &mut B {
        &mut self.board
    }

    fn player(&self) -> Player {
        self.data.player
    }

    fn update_player(&mut self, player: Player) {
        self.data.player = player;
        if let Some(hash) = self.data.hash.as_mut() {
            hash.switch_turn();
        }
    }

    fn info(&self) -> &GameInfo {
        &self.data.info
    }

    fn last_move(&self) -> &Option<MoveInfo> {
        &self.data.last_move
    }

    fn do_move_no_checks(&mut self, move_action: &MoveAction) -> Result<()> {
        let mv = &move_action.mv;

        let Some(source_square) = self.board.at(&mv.source) else {
            bail!("Move {} from empty square:\n{}", mv, self.board);
        };

        let player = source_square.player;
        let moved_piece = source_square.piece;
        let move_info = match moved_piece {
            PieceType::Pawn => {
                if mv.source.rank.abs_diff(mv.target.rank) == 2 {
                    MoveExtraInfo::Passed
                } else if mv.source.file != mv.target.file && self.board.at(&mv.target).is_none() {
                    MoveExtraInfo::EnPassant
                } else if mv.target.rank == B::promotion_rank(&player) {
                    let promotion_piece = match move_action.move_type {
                        MoveActionType::Normal => bail!("Promotion piece not specified"),
                        MoveActionType::Promotion(piece) => piece,
                    };

                    MoveExtraInfo::Promotion(promotion_piece)
                } else {
                    MoveExtraInfo::Other
                }
            }
            PieceType::King => {
                if mv.source.file.abs_diff(mv.target.file) == 2 {
                    match mv.target.file {
                        2 => MoveExtraInfo::CastleQueenside,
                        6 => MoveExtraInfo::CastleKingside,
                        _ => bail!("invalid castling {} in:\n{}", mv, self.board),
                    }
                } else {
                    MoveExtraInfo::Other
                }
            }
            _ => MoveExtraInfo::Other,
        };

        let captured = self.board.at(&mv.target);

        self.move_piece(&mv.source, &mv.target);

        match move_info {
            MoveExtraInfo::Passed => {
                if let Some(hash) = self.data.hash.as_mut() {
                    hash.switch_en_passant_file(mv.source.file);
                }
            }
            MoveExtraInfo::EnPassant => {
                // Capture passed pawn
                let direction = B::pawn_progress_direction(&player);
                let passed = self
                    .try_move(&mv.target, &dir!(-direction, 0))
                    .only_enemy(player)
                    .next()
                    .unwrap();
                self.update(&passed, None);
            }
            MoveExtraInfo::Promotion(promotion_piece) => {
                self.update(
                    &mv.target,
                    Some(Piece {
                        piece: promotion_piece.into(),
                        player,
                    }),
                );
            }
            MoveExtraInfo::CastleKingside => {
                let rook_source = self.try_move(&mv.source, &dir!(0, 3)).next().unwrap();
                let rook_target = self.try_move(&mv.source, &dir!(0, 1)).next().unwrap();
                self.move_piece(&rook_source, &rook_target);
            }
            MoveExtraInfo::CastleQueenside => {
                let rook_source = self.try_move(&mv.source, &dir!(0, -4)).next().unwrap();
                let rook_target = self.try_move(&mv.source, &dir!(0, -1)).next().unwrap();
                self.move_piece(&rook_source, &rook_target);
            }
            _ => (),
        }

        if moved_piece == PieceType::King {
            self.disable_castle_kingside(player);
            self.disable_castle_queenside(player);
        } else if moved_piece == PieceType::Rook && mv.source.rank == B::home_rank(&player) {
            match mv.source.file {
                0 => self.disable_castle_queenside(player),
                7 => self.disable_castle_kingside(player),
                _ => (),
            }
        }

        if let Some(captured) = captured {
            if captured.piece == PieceType::Rook && mv.target.rank == B::home_rank(&captured.player) {
                match mv.target.file {
                    0 => self.disable_castle_queenside(captured.player),
                    7 => self.disable_castle_kingside(captured.player),
                    _ => (),
                }
            }
        }

        if let Some(last_move) = self.data.last_move {
            if last_move.info == MoveExtraInfo::Passed {
                if let Some(hash) = self.data.hash.as_mut() {
                    hash.switch_en_passant_file(last_move.mv.source.file);
                }
            }
        }

        self.update_player(!self.data.player);
        self.data.last_move = Some(MoveInfo {
            mv: *mv,
            info: move_info,
        });

        Ok(())
    }
}

impl<B: Board> IterableBoard for GameState<B> {}
