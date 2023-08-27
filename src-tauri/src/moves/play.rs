use crate::board::{Board, Game, Move, MoveExtraInfo, MoveInfo, Piece, PieceType, Player};
use crate::moves::conditions::{enemy, only_enemy, try_move, Direction};
use crate::moves::iter::dir;
use crate::mv;
use std::marker::PhantomData;

use super::get_possible_moves_no_checks;

pub struct ReversableMove {
    mv: Move,
    previous_piece: Option<Piece>,
}

pub trait PlayableGame<'a> {
    fn from_game(game: &'a mut Game) -> Self;
    fn as_ref(&self) -> &Game;
    fn as_mut(&mut self) -> &mut Game;

    fn push_moves(&mut self, moves: &mut Vec<ReversableMove>);

    fn do_move(&mut self, mv: &Move) -> bool {
        let board = &self.as_ref().board;

        match board.square(&mv.source) {
            Some(piece) => {
                if piece.player != self.as_ref().player {
                    return false;
                }
            }
            None => {
                return false;
            }
        }

        let possible_moves =
            get_possible_moves_no_checks(&board, &self.as_ref().last_move, mv.source);

        if possible_moves
            .iter()
            .find(|possible_position| mv.target == **possible_position)
            .is_none()
        {
            return false;
        }

        self.do_move_no_checks(mv)
    }

    fn do_move_no_checks(&mut self, mv: &Move) -> bool {
        let board = &mut self.as_mut().board;
        let player = board.square(&mv.source).unwrap().player;
        let moved_piece = board.square(&mv.source).unwrap().piece;
        let move_info = match moved_piece {
            PieceType::Pawn => {
                if mv.source.row.abs_diff(mv.target.row) == 2 {
                    MoveExtraInfo::Passed
                } else if mv.source.col != mv.target.col && board.square(&mv.target).is_none() {
                    MoveExtraInfo::EnPassant
                } else if mv.target.row == Board::promotion_rank(&player) {
                    MoveExtraInfo::Promotion(PieceType::Queen)
                } else {
                    MoveExtraInfo::Other
                }
            }
            _ => MoveExtraInfo::Other,
        };
        let mut moves: Vec<ReversableMove> = Vec::new();

        moves.push(ReversableMove {
            mv: *mv,
            previous_piece: *board.square(&mv.target),
        });

        board.move_piece(&mv.source, &mv.target);

        match move_info {
            MoveExtraInfo::EnPassant => {
                // Capture passed pawn
                let direction: i8 = match player {
                    Player::White => 1,
                    Player::Black => -1,
                };
                let passed =
                    only_enemy(&board, try_move(&mv.target, &dir!(-direction, 0)), &player)
                        .unwrap();

                moves.push(ReversableMove {
                    mv: mv!(passed, passed),
                    previous_piece: *board.square(&passed),
                });

                board.update(&passed, None);
            }
            MoveExtraInfo::Promotion(piece) => {
                moves.push(ReversableMove {
                    mv: mv!(mv.target, mv.target),
                    previous_piece: *board.square(&mv.target),
                });
                board.update(&mv.target, Some(Piece { piece, player }));
            }
            _ => (),
        }

        self.as_mut().player = enemy(&self.as_ref().player);
        self.push_moves(&mut moves);
        self.as_mut().last_move = Some(MoveInfo {
            mv: *mv,
            info: move_info,
        });

        true
    }
}

pub struct SearchableGame<'a> {
    game: Game,
    _marker: PhantomData<&'a ()>,
}

impl<'a> PlayableGame<'a> for SearchableGame<'a> {
    fn from_game(game: &'a mut Game) -> Self {
        SearchableGame {
            game: *game,
            _marker: PhantomData,
        }
    }

    fn as_ref(&self) -> &Game {
        &self.game
    }

    fn as_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    fn push_moves(&mut self, _moves: &mut Vec<ReversableMove>) {}
}

pub struct ReversableGame<'a> {
    game: &'a mut Game,
    moves: Vec<ReversableMove>,
    last_move: Option<MoveInfo>,
    move_player: Player,
}

impl<'a> PlayableGame<'a> for ReversableGame<'a> {
    fn from_game(game: &'a mut Game) -> Self {
        let last_move = game.last_move;
        let player = game.player;
        ReversableGame {
            game,
            moves: vec![],
            last_move: last_move,
            move_player: player,
        }
    }

    fn as_ref(&self) -> &Game {
        &self.game
    }

    fn as_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    fn push_moves(&mut self, moves: &mut Vec<ReversableMove>) {
        self.moves.append(moves);
    }
}

impl<'a> ReversableGame<'a> {
    pub fn undo(&mut self) {
        assert!(!self.moves.is_empty());

        for rev_move in self.moves.iter().rev() {
            let mv = &rev_move.mv;

            self.game.board.move_piece(&mv.target, &mv.source);
            self.game.board.update(&mv.target, rev_move.previous_piece);
        }

        self.moves.clear();
        self.game.player = enemy(&self.game.player);
        self.game.last_move = self.last_move;
        self.last_move = None;
    }
}
