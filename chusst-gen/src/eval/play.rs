use crate::board::{Board, ModifiableBoard, Piece, Position};
use crate::eval::Game;
use crate::game::{
    CastlingRights, GameInfo, GameState, ModifiableGame, Move, MoveAction, MoveInfo,
};
use crate::mv;
use anyhow::Result;

use super::check::SafetyChecks;

pub struct ReversableMove {
    mv: Move,
    previous_piece: Option<Piece>,
}

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

pub struct ReversableGame<'a, B: Board> {
    game: &'a mut GameState<B>,
    moves: Vec<ReversableMove>,
    last_move: Option<MoveInfo>,
    info: Option<GameInfo>,
}

impl<'a, B: Board> CastlingRights for ReversableGame<'a, B> {
    fn can_castle_kingside(&self, player: crate::board::Player) -> bool {
        self.game.can_castle_kingside(player)
    }

    fn can_castle_queenside(&self, player: crate::board::Player) -> bool {
        self.game.can_castle_queenside(player)
    }

    fn disable_castle_kingside(&mut self, player: crate::board::Player) {
        self.game.disable_castle_kingside(player);
    }

    fn disable_castle_queenside(&mut self, player: crate::board::Player) {
        self.game.disable_castle_queenside(player);
    }
}

impl<'a, B: Board + SafetyChecks> ModifiableBoard<Position, Option<Piece>>
    for ReversableGame<'a, B>
{
    fn at(&self, pos: &Position) -> Option<Piece> {
        self.game.at(pos)
    }

    fn move_piece(&mut self, source: &Position, target: &Position) {
        let mv = mv!(*source, *target);
        self.moves.push(ReversableMove {
            mv,
            previous_piece: self.game.at(&mv.target),
        });
        self.game.move_piece(&mv.source, &mv.target);
    }

    fn update(&mut self, pos: &Position, value: Option<Piece>) {
        self.moves.push(ReversableMove {
            mv: mv!(*pos, *pos),
            previous_piece: self.game.at(pos),
        });
        self.game.update(pos, value);
    }
}

impl<'a, B: Board + SafetyChecks> ModifiableGame<B> for ReversableGame<'a, B> {
    fn board(&self) -> &B {
        self.board()
    }

    fn board_mut(&mut self) -> &mut B {
        self.board_mut()
    }

    fn player(&self) -> crate::board::Player {
        self.player()
    }

    fn update_player(&mut self, player: crate::board::Player) {
        self.update_player(player)
    }

    fn info(&self) -> &GameInfo {
        self.game.info()
    }

    fn do_move_no_checks(&mut self, move_action: &MoveAction) -> Result<()> {
        let mut moves: Vec<ReversableMove> = Vec::new();
        ModifiableGame::do_move_no_checks(self, move_action)?;
        self.moves.append(&mut moves);
        Ok(())
    }
}

impl<'a, B: Board> ReversableGame<'a, B> {
    #[allow(dead_code)]
    pub fn from(game: &'a mut GameState<B>) -> Self {
        let last_move = game.last_move();
        let game_info = game.info();
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

            self.game.move_piece(&mv.target, &mv.source);
            self.game.update(&mv.target, rev_move.previous_piece);
        }

        self.moves.clear();
        self.game.player = !self.game.player;
        self.game.last_move = self.last_move;
        self.game.info = self.info.unwrap();
        self.last_move = None;
        self.info = None;
    }
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
        ModifiableGame::do_move_no_checks(self, mv)?;
        self.moves.append(&mut moves);
        Ok(())
    }
}
