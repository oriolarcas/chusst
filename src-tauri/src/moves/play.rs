use super::get_possible_moves;
use crate::board::{
    Board, Game, GameInfo, Move, MoveExtraInfo, MoveInfo, Piece, PieceType, Player,
};
use crate::moves::conditions::{enemy, only_enemy, try_move, Direction};
use crate::moves::iter::dir;
use crate::mv;
use std::marker::PhantomData;

pub struct ReversableMove {
    mv: Move,
    previous_piece: Option<Piece>,
}

pub trait PlayableGame<'a> {
    fn from_game(game: &'a mut Game) -> Self;
    fn as_ref(&self) -> &Game;
    fn as_mut(&mut self) -> &mut Game;

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

        let possible_moves = get_possible_moves(
            &board,
            &self.as_ref().last_move,
            &self.as_ref().info,
            mv.source,
        );

        if possible_moves
            .iter()
            .find(|possible_position| mv.target == **possible_position)
            .is_none()
        {
            return false;
        }

        self.do_move_no_checks(mv);

        true
    }

    fn do_move_no_checks(&mut self, mv: &Move);
}

trait PlayableGameNoChecksImpl<'a>: PlayableGame<'a> {
    fn do_move_no_checks_impl(
        &mut self,
        mv: &Move,
        moves_option: &mut Option<&mut Vec<ReversableMove>>,
    ) {
        let board = &mut self.as_mut().board;

        let square_opt = board.square(&mv.source).as_ref();
        assert!(
            square_opt.is_some(),
            "move {} from empty square:\n{}",
            mv,
            board
        );
        let square = square_opt.unwrap();

        let player = square.player;
        let moved_piece = square.piece;
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
            PieceType::King => {
                if mv.source.col.abs_diff(mv.target.col) == 2 {
                    match mv.target.col {
                        2 => MoveExtraInfo::CastleQueenside,
                        6 => MoveExtraInfo::CastleKingside,
                        _ => panic!("invalid castling {} in:\n{}", mv, board),
                    }
                } else {
                    MoveExtraInfo::Other
                }
            }
            _ => MoveExtraInfo::Other,
        };

        if let Some(moves) = moves_option {
            moves.push(ReversableMove {
                mv: *mv,
                previous_piece: *board.square(&mv.target),
            });
        }

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

                if let Some(moves) = moves_option {
                    moves.push(ReversableMove {
                        mv: mv!(passed, passed),
                        previous_piece: *board.square(&passed),
                    });
                }

                board.update(&passed, None);
            }
            MoveExtraInfo::Promotion(piece) => {
                if let Some(moves) = moves_option {
                    moves.push(ReversableMove {
                        mv: mv!(mv.target, mv.target),
                        previous_piece: *board.square(&mv.target),
                    });
                }
                board.update(&mv.target, Some(Piece { piece, player }));
            }
            MoveExtraInfo::CastleKingside => {
                let rook_source = try_move(&mv.source, &dir!(0, 3)).unwrap();
                let rook_target = try_move(&mv.source, &dir!(0, 1)).unwrap();
                if let Some(moves) = moves_option {
                    moves.push(ReversableMove {
                        mv: mv!(rook_source, rook_target),
                        previous_piece: None,
                    });
                }
                board.move_piece(&rook_source, &rook_target);
            }
            MoveExtraInfo::CastleQueenside => {
                let rook_source = try_move(&mv.source, &dir!(0, -4)).unwrap();
                let rook_target = try_move(&mv.source, &dir!(0, -1)).unwrap();
                if let Some(moves) = moves_option {
                    moves.push(ReversableMove {
                        mv: mv!(rook_source, rook_target),
                        previous_piece: None,
                    });
                }
                board.move_piece(&rook_source, &rook_target);
            }
            _ => (),
        }

        if moved_piece == PieceType::King {
            self.as_mut().info.disable_castle_kingside(&player);
            self.as_mut().info.disable_castle_queenside(&player);
        } else if moved_piece == PieceType::Rook && mv.source.row == Board::home_rank(&player) {
            match mv.source.col {
                0 => self.as_mut().info.disable_castle_queenside(&player),
                7 => self.as_mut().info.disable_castle_kingside(&player),
                _ => (),
            }
        }

        self.as_mut().player = enemy(&self.as_ref().player);
        self.as_mut().last_move = Some(MoveInfo {
            mv: *mv,
            info: move_info,
        });
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

    fn do_move_no_checks(&mut self, mv: &Move) {
        self.do_move_no_checks_impl(mv, &mut None)
    }
}

impl<'a> PlayableGameNoChecksImpl<'a> for SearchableGame<'a> {}

pub struct ReversableGame<'a> {
    game: &'a mut Game,
    moves: Vec<ReversableMove>,
    last_move: Option<MoveInfo>,
    info: Option<GameInfo>,
}

impl<'a> PlayableGame<'a> for ReversableGame<'a> {
    fn from_game(game: &'a mut Game) -> Self {
        let last_move = game.last_move;
        let game_info = game.info;
        ReversableGame {
            game,
            moves: vec![],
            last_move,
            info: Some(game_info),
        }
    }

    fn as_ref(&self) -> &Game {
        &self.game
    }

    fn as_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    fn do_move_no_checks(&mut self, mv: &Move) {
        let mut moves: Vec<ReversableMove> = Vec::new();
        let result = self.do_move_no_checks_impl(mv, &mut Some(&mut moves));
        self.moves.append(&mut moves);
        result
    }
}

impl<'a> PlayableGameNoChecksImpl<'a> for ReversableGame<'a> {}

impl<'a> ReversableGame<'a> {
    #[allow(dead_code)]
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
        self.game.info = self.info.unwrap();
        self.last_move = None;
        self.info = None;
    }
}
