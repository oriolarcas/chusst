use crate::game::{GameHash, MoveAction, SimpleGame};
use anyhow::{Context, Result};
use std::collections::HashMap;

use super::Game;

pub type GameHistory = Vec<MoveAction>;

#[derive(Default)]
pub struct HashedHistory {
    moves: Vec<(MoveAction, GameHash)>,
    hashes: HashMap<GameHash, Vec<usize>>,
}

impl HashedHistory {
    pub fn from(moves: &GameHistory) -> Result<Self> {
        let mut game = SimpleGame::new();
        let mut history = Self::default();

        // Hash of the initial position is always added, even if no moves are made
        history.hashes.insert(game.hash(), Vec::new());

        for mv in moves {
            let hash = game.hash();
            history.push(*mv, hash);
            game.do_move(mv)
                .context(format!("Invalid move {}", mv.mv))?;
        }

        Ok(history)
    }

    pub fn push(&mut self, mv: MoveAction, hash: GameHash) {
        self.moves.push((mv, hash));
        self.hashes
            .entry(hash)
            .or_default()
            .push(self.moves.len() - 1);
    }

    pub fn pop(&mut self) -> Result<MoveAction> {
        let (mv, hash) = self.moves.pop().context("Empty")?;
        self.hashes
            .get_mut(&hash)
            .context("Hash not found")?
            .pop()
            .context("No moves for this hash")?;
        Ok(mv)
    }

    pub fn count(&self, hash: &GameHash) -> usize {
        self.hashes.get(hash).map_or(0, |v| v.len())
    }
}
