use crate::game::{GameHash, GameHashBuilder, MoveAction, SimpleGame};
use anyhow::{Context, Result};
use std::collections::HashMap;

use super::Game;

pub type GameHistory = Vec<MoveAction>;

pub struct HashedHistory {
    moves: Vec<(MoveAction, GameHash)>,
    hashes: HashMap<GameHash, Vec<usize>, GameHashBuilder>,
    // hashes: HashMap<GameHash, Vec<usize>>,
}

impl Default for HashedHistory {
    fn default() -> Self {
        Self {
            moves: Vec::new(),
            hashes: HashMap::with_hasher(GameHashBuilder),
            // hashes: HashMap::new(),
        }
    }
}

impl HashedHistory {
    pub fn from(moves: &GameHistory) -> Result<Self> {
        let mut game = SimpleGame::new();
        let mut history = Self::default();

        history.reserve(moves.len() + 1);

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

    pub fn reserve(&mut self, additional: usize) {
        self.moves.reserve(additional);
        self.hashes.reserve(additional);
    }

    pub fn push(&mut self, mv: MoveAction, hash: GameHash) {
        self.moves.push((mv, hash));
        self.hashes
            .entry(hash)
            .or_insert(Vec::with_capacity(2))
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
