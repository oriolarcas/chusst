use crate::board::Board;
use crate::eval::Game;
use crate::game::{GameState, ModifiableGame, MoveAction};
use anyhow::Result;

use super::check::SafetyChecks;

pub trait PlayableGame<B: Board>: ModifiableGame<B>
where
    B: SafetyChecks,
{
    fn as_ref(&self) -> &GameState<B>;
    fn as_mut(&mut self) -> &mut GameState<B>;

    fn clone_and_move(&self, mv: &MoveAction) -> Result<GameState<B>> {
        let mut new_game = self.as_ref().clone();
        PlayableGame::do_move_no_checks(&mut new_game, mv)?;
        Ok(new_game)
    }

    // This is slower than using eval::GamePrivate::clone_and_move_with_checks()
    // because here we don't know if the move is legal, so we check against
    // all possible legal moves.
    fn do_move_with_checks(&mut self, move_action: &MoveAction) -> bool {
        let mv = &move_action.mv;

        match self.at(&mv.source) {
            Some(piece) => {
                if piece.player != self.player() {
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

        PlayableGame::do_move_no_checks(self, move_action).is_ok()
    }

    fn do_move_no_checks(&mut self, move_action: &MoveAction) -> Result<()>;
}
