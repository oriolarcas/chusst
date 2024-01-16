use crate::board::{Board, ModifiableBoard, Piece, PieceType, Position};
use crate::eval::conditions::{only_enemy, try_move, Direction};
// use crate::eval::get_possible_moves;
use crate::eval::iter::dir;
use crate::eval::Game;
use crate::game::{GameInfo, GameState, Move, MoveAction, MoveActionType, MoveExtraInfo, MoveInfo};
use crate::mv;
use anyhow::{bail, Result};

use super::check::SafetyChecks;

pub struct ReversableMove {
    mv: Move,
    previous_piece: Option<Piece>,
}

pub trait PlayableGame<B: Board>: ModifiableBoard<Position, Option<Piece>>
where
    B: SafetyChecks,
{
    fn as_ref(&self) -> &GameState<B>;
    fn as_mut(&mut self) -> &mut GameState<B>;

    fn clone_and_move(&self, mv: &MoveAction) -> Result<GameState<B>> {
        let mut new_game = self.as_ref().clone();
        new_game.do_move_no_checks(mv)?;
        Ok(new_game)
    }

    // This is slower than using eval::GamePrivate::clone_and_move_with_checks()
    // because here we don't know if the move is legal, so we check against
    // all possible legal moves.
    fn do_move_with_checks(&mut self, move_action: &MoveAction) -> bool {
        let board = &self.as_ref().board;
        let mv = &move_action.mv;

        match board.at(&mv.source) {
            Some(piece) => {
                if piece.player != self.as_ref().player {
                    return false;
                }
            }
            None => {
                return false;
            }
        }

        let possible_moves = self.as_ref().get_possible_moves(mv.source);

        if !possible_moves
            .iter()
            .any(|possible_move| mv.target == possible_move.mv.target)
        {
            return false;
        }

        self.do_move_no_checks(move_action).is_ok()
    }

    fn do_move_no_checks(&mut self, move_action: &MoveAction) -> Result<()>;

    fn do_move_no_checks_internal(&mut self, move_action: &MoveAction) -> Result<()> {
        let mv = &move_action.mv;

        let Some(source_square) = self.as_ref().board.at(&mv.source) else {
            bail!("Move {} from empty square:\n{}", mv, self.as_ref().board);
        };

        let player = source_square.player;
        let moved_piece = source_square.piece;
        let move_info = match moved_piece {
            PieceType::Pawn => {
                if mv.source.rank.abs_diff(mv.target.rank) == 2 {
                    MoveExtraInfo::Passed
                } else if mv.source.file != mv.target.file
                    && self.as_ref().board.at(&mv.target).is_none()
                {
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
                        _ => bail!("invalid castling {} in:\n{}", mv, self.as_ref().board),
                    }
                } else {
                    MoveExtraInfo::Other
                }
            }
            _ => MoveExtraInfo::Other,
        };

        self.move_piece(&mv.source, &mv.target);

        match move_info {
            MoveExtraInfo::EnPassant => {
                // Capture passed pawn
                let direction = B::pawn_progress_direction(&player);
                let passed = only_enemy(
                    &self.as_ref().board,
                    try_move(&mv.target, &dir!(-direction, 0)),
                    &player,
                )
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
                let rook_source = try_move(&mv.source, &dir!(0, 3)).unwrap();
                let rook_target = try_move(&mv.source, &dir!(0, 1)).unwrap();
                self.move_piece(&rook_source, &rook_target);
            }
            MoveExtraInfo::CastleQueenside => {
                let rook_source = try_move(&mv.source, &dir!(0, -4)).unwrap();
                let rook_target = try_move(&mv.source, &dir!(0, -1)).unwrap();
                self.move_piece(&rook_source, &rook_target);
            }
            _ => (),
        }

        if moved_piece == PieceType::King {
            self.as_mut().info.disable_castle_kingside(&player);
            self.as_mut().info.disable_castle_queenside(&player);
        } else if moved_piece == PieceType::Rook && mv.source.rank == B::home_rank(&player) {
            match mv.source.file {
                0 => self.as_mut().info.disable_castle_queenside(&player),
                7 => self.as_mut().info.disable_castle_kingside(&player),
                _ => (),
            }
        }

        self.as_mut().player = !self.as_ref().player;
        self.as_mut().last_move = Some(MoveInfo {
            mv: *mv,
            info: move_info,
        });

        Ok(())
    }
}

pub struct ReversableGame<'a, B: Board> {
    game: &'a mut GameState<B>,
    moves: Vec<ReversableMove>,
    last_move: Option<MoveInfo>,
    info: Option<GameInfo>,
}

impl<'a, B: Board + SafetyChecks> PlayableGame<B> for ReversableGame<'a, B> {
    fn as_ref(&self) -> &GameState<B> {
        self.game
    }

    fn as_mut(&mut self) -> &mut GameState<B> {
        self.game
    }

    fn do_move_no_checks(&mut self, mv: &MoveAction) -> Result<()> {
        let mut moves: Vec<ReversableMove> = Vec::new();
        PlayableGame::do_move_no_checks_internal(self, mv)?;
        self.moves.append(&mut moves);
        Ok(())
    }
}

impl<'a, B: Board + SafetyChecks> ModifiableBoard<Position, Option<Piece>>
    for ReversableGame<'a, B>
{
    fn at(&self, pos: &Position) -> Option<Piece> {
        self.game.board.at(pos)
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        let mv = mv!(*source, *target);
        self.moves.push(ReversableMove {
            mv,
            previous_piece: self.as_ref().board.at(&mv.target),
        });
        self.as_mut().board.move_piece(&mv.source, &mv.target);
    }

    fn update(&mut self, pos: &Position, value: Option<Piece>) {
        self.moves.push(ReversableMove {
            mv: mv!(*pos, *pos),
            previous_piece: self.as_ref().board.at(pos),
        });
        self.as_mut().board.update(pos, value);
    }
}

impl<'a, B: Board> ReversableGame<'a, B> {
    #[allow(dead_code)]
    pub fn from(game: &'a mut GameState<B>) -> Self {
        let last_move = game.last_move;
        let game_info = game.info;
        ReversableGame {
            game,
            moves: vec![],
            last_move,
            info: Some(game_info),
        }
    }

    // undo() only used in tests
    #[allow(dead_code)]
    pub fn undo(&mut self) {
        assert!(!self.moves.is_empty());

        for rev_move in self.moves.iter().rev() {
            let mv = &rev_move.mv;

            self.game.board.move_piece(&mv.target, &mv.source);
            self.game.board.update(&mv.target, rev_move.previous_piece);
        }

        self.moves.clear();
        self.game.player = !self.game.player;
        self.game.last_move = self.last_move;
        self.game.info = self.info.unwrap();
        self.last_move = None;
        self.info = None;
    }
}
