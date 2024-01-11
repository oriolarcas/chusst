use crate::board::{Board, ModifiableBoard, Piece, PieceType, Position, Square};
use crate::eval::conditions::{enemy, only_enemy, try_move, Direction};
use crate::eval::get_possible_moves;
use crate::eval::iter::dir;
use crate::game::{Game, GameInfo, Move, MoveAction, MoveActionType, MoveExtraInfo, MoveInfo};
use crate::mv;

use self::internal_searchable_game::InternalSearchableGame;

pub struct ReversableMove {
    mv: Move,
    previous_piece: Option<Piece>,
}

pub trait PlayableGame<'a> {
    fn as_ref(&self) -> &Game;
    fn as_mut(&mut self) -> &mut Game;

    fn do_move(&mut self, move_action: &MoveAction) -> bool {
        let board = &self.as_ref().board;
        let mv = &move_action.mv;

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
            .find(|possible_move| mv.target == possible_move.mv.target)
            .is_none()
        {
            return false;
        }

        self.do_move_no_checks(move_action);

        true
    }

    fn do_move_no_checks(&mut self, mv: &MoveAction);
}

trait PlayableGamePrivate<'a>: PlayableGame<'a> + ModifiableBoard {
    fn do_move_no_checks_private(&mut self, move_action: &MoveAction) {
        let mv = &move_action.mv;

        let Some(source_square) = self.as_ref().board.square(&mv.source) else {
            panic!("Move {} from empty square:\n{}", mv, self.as_ref().board);
        };

        let player = source_square.player;
        let moved_piece = source_square.piece;
        let move_info = match moved_piece {
            PieceType::Pawn => {
                if mv.source.rank.abs_diff(mv.target.rank) == 2 {
                    MoveExtraInfo::Passed
                } else if mv.source.file != mv.target.file
                    && self.as_ref().board.square(&mv.target).is_none()
                {
                    MoveExtraInfo::EnPassant
                } else if mv.target.rank == Board::promotion_rank(&player) {
                    let promotion_piece = match move_action.move_type {
                        MoveActionType::Normal => panic!("Promotion piece not specified"),
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
                        _ => panic!("invalid castling {} in:\n{}", mv, self.as_ref().board),
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
                let direction = Board::pawn_progress_direction(&player);
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
        } else if moved_piece == PieceType::Rook && mv.source.rank == Board::home_rank(&player) {
            match mv.source.file {
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

#[cfg(not(feature = "bitboards"))]
mod internal_searchable_game {
    use super::*;

    #[derive(Clone)]
    pub struct InternalSearchableGame(Game);

    impl InternalSearchableGame {
        pub fn as_ref(&self) -> &Game {
            &self.0
        }

        pub fn as_mut(&mut self) -> &mut Game {
            &mut self.0
        }
    }

    impl From<&Game> for InternalSearchableGame {
        fn from(value: &Game) -> Self {
            InternalSearchableGame(value.clone())
        }
    }

    impl ModifiableBoard for InternalSearchableGame {
        fn move_piece(&mut self, _source: &Position, _target: &Position) {
            // do nothing
        }

        fn update(&mut self, _pos: &Position, _value: Square) {
            // do nothing
        }
    }
}

#[cfg(feature = "bitboards")]
mod internal_searchable_game {
    use crate::eval::bitboards::BitboardGame;

    pub type InternalSearchableGame = BitboardGame;
}

pub struct SearchableGame {
    game: internal_searchable_game::InternalSearchableGame,
}

impl SearchableGame {
    pub fn clone_and_move(&self, mv: &MoveAction) -> SearchableGame {
        let mut new_game = SearchableGame {
            game: self.game.clone(),
        };

        new_game.do_move_no_checks(mv);

        new_game
    }

    pub fn as_ref(&self) -> &Game {
        &self.game.as_ref()
    }

    pub fn as_mut(&mut self) -> &mut Game {
        self.game.as_mut()
    }

    #[cfg(feature = "bitboards")]
    pub fn bitboards_by_player(
        &self,
        player: &crate::board::Player,
    ) -> &super::bitboards::PlayerBitboards {
        &self.game.by_player(player)
    }
}

impl<'a> PlayableGame<'a> for SearchableGame {
    fn as_ref(&self) -> &Game {
        &self.game.as_ref()
    }

    fn as_mut(&mut self) -> &mut Game {
        self.game.as_mut()
    }

    fn do_move_no_checks(&mut self, mv: &MoveAction) {
        self.do_move_no_checks_private(mv);
    }
}

impl From<&Game> for SearchableGame {
    fn from(game: &Game) -> SearchableGame {
        SearchableGame {
            game: InternalSearchableGame::from(game),
        }
    }
}

impl ModifiableBoard for SearchableGame {
    fn move_piece(&mut self, source: &Position, target: &Position) {
        let mv = mv!(*source, *target);
        self.game.move_piece(&mv.source, &mv.target);
        self.as_mut().board.move_piece(&mv.source, &mv.target);
    }

    fn update(&mut self, pos: &Position, value: Square) {
        self.game.update(pos, value);
        self.as_mut().board.update(&pos, value);
    }
}

impl<'a> PlayableGamePrivate<'a> for SearchableGame {}

pub struct ReversableGame<'a> {
    game: &'a mut Game,
    moves: Vec<ReversableMove>,
    last_move: Option<MoveInfo>,
    info: Option<GameInfo>,
}

impl<'a> PlayableGame<'a> for ReversableGame<'a> {
    fn as_ref(&self) -> &Game {
        &self.game
    }

    fn as_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    fn do_move_no_checks(&mut self, mv: &MoveAction) {
        let mut moves: Vec<ReversableMove> = Vec::new();
        let result = self.do_move_no_checks_private(mv);
        self.moves.append(&mut moves);
        result
    }
}

impl<'a> ModifiableBoard for ReversableGame<'a> {
    fn move_piece(&mut self, source: &Position, target: &Position) {
        let mv = mv!(*source, *target);
        self.moves.push(ReversableMove {
            mv,
            previous_piece: self.as_ref().board.square(&mv.target),
        });
        self.as_mut().board.move_piece(&mv.source, &mv.target);
    }

    fn update(&mut self, pos: &Position, value: Square) {
        self.moves.push(ReversableMove {
            mv: mv!(*pos, *pos),
            previous_piece: self.as_ref().board.square(pos),
        });
        self.as_mut().board.update(&pos, value);
    }
}

impl<'a> PlayableGamePrivate<'a> for ReversableGame<'a> {}

impl<'a> ReversableGame<'a> {
    pub fn from(game: &'a mut Game) -> Self {
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
        self.game.player = enemy(&self.game.player);
        self.game.last_move = self.last_move;
        self.game.info = self.info.unwrap();
        self.last_move = None;
        self.info = None;
    }
}
