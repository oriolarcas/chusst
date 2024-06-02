mod check;
mod conditions;
mod feedback;
mod history;
mod iter;
mod play;

#[cfg(test)]
mod tests;

use self::check::{only_empty_and_safe, SafetyChecks};
use self::feedback::SearchFeedback;
pub use self::feedback::{
    EngineFeedback, EngineFeedbackMessage, EngineMessage, PeriodicalSearchFeedback,
    SearchTreeFeedback, SilentSearchFeedback, StdoutFeedback,
};
pub use self::history::GameHistory;
use self::history::HashedHistory;
pub use self::iter::dir;
use self::iter::piece_into_iter;
use self::play::PlayableGame;
use crate::board::{Board, Direction, Piece, PieceType, Player, Position, PositionIterator, Ranks};
use crate::game::{GameState, ModifiableGame, Move, MoveAction, MoveActionType, PromotionPieces};
use crate::{mv, mva, pos};

use anyhow::{bail, Result};
use core::{fmt, panic};
use serde::Serialize;

use std::collections::HashMap;
use std::time::Instant;

pub trait HasStopSignal {
    fn stop(&mut self) -> bool;
}

impl HasStopSignal for () {
    fn stop(&mut self) -> bool {
        false
    }
}

macro_rules! log {
    ($logger:expr, $str:expr) => {
        {
            let _ = writeln!($logger, $str);
        }
    };
    ($logger:expr, $fmt:expr, $($param:expr),*) => {
        {
            let _ = writeln!($logger, $fmt, $($param),+);
        }
    };
}

// List of pieces that can capture each square
pub type BoardCaptures = Ranks<Vec<Position>>;

// Score in centipawns
#[derive(PartialEq, Default, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Score(i32);

// Value in centipawns
impl Score {
    pub const MAX: Self = Self(i32::MAX);
    pub const MIN: Self = Self(-i32::MAX); // -i32::MIN > i32::MAX

    pub fn piece_value(piece: PieceType) -> Score {
        match piece {
            PieceType::Pawn => Score::from(100),
            PieceType::Knight => Score::from(300),
            PieceType::Bishop => Score::from(300),
            PieceType::Rook => Score::from(500),
            PieceType::Queen => Score::from(900),
            PieceType::King => Score::MAX,
        }
    }

    // Score lower than losing any piece, but higher than stalemate
    pub fn stalemate() -> Score {
        Score::from(Self::MIN.0 / 2)
    }
}

impl From<i32> for Score {
    fn from(value: i32) -> Self {
        Score(value)
    }
}

impl From<Score> for i32 {
    fn from(val: Score) -> Self {
        val.0
    }
}

impl std::fmt::Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if *self <= Self::MIN {
            write!(f, "-∞")
        } else if *self >= Self::MAX {
            write!(f, "+∞")
        } else {
            self.0.fmt(f)
        }
    }
}

impl std::ops::Add for Score {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self::Output::from(self.0.saturating_add(other.0))
    }
}

impl std::ops::Sub for Score {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self::Output::from(self.0.saturating_sub(other.0))
    }
}

impl std::ops::Neg for Score {
    type Output = Self;

    fn neg(self) -> Self::Output {
        if self <= Self::MIN {
            Self::MAX
        } else {
            Self(-self.0)
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum MateType {
    Checkmate,
    Stalemate,
}

#[derive(Copy, Clone)]
pub enum GameMove {
    Normal(MoveAction),
    Mate(MateType),
}

#[derive(PartialEq)]
pub struct WeightedMove {
    pub mv: MoveAction,
    pub score: Score,
}

#[derive(Copy, Clone, PartialEq)]
pub enum GameResult {
    Win(Player),
    Draw,
}

#[derive(Default)]
pub struct Branch {
    pub moves: Vec<WeightedMove>,
    pub score: Score,
    pub searched: u32,
    pub result: Option<GameResult>,
}

impl fmt::Display for Branch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} = {:+}",
            self.moves
                .iter()
                .map(|mv| format!("{}{}{:+}", mv.mv.mv.source, mv.mv.mv.target, mv.score))
                .collect::<Vec<String>>()
                .join(" "),
            self.score
        )
    }
}

struct SearchResult {
    branch: Option<Branch>,
    stopped: bool,
}

struct SearchScores {
    parent: Score,
    alpha: Score,
    beta: Score,
}

impl Default for SearchScores {
    fn default() -> Self {
        Self {
            parent: Score::from(0),
            alpha: Score::MIN,
            beta: Score::MAX,
        }
    }
}

impl<B: Board + SafetyChecks> PlayableGame<B> for GameState<B> {
    fn as_ref(&self) -> &GameState<B> {
        self
    }

    fn as_mut(&mut self) -> &mut GameState<B> {
        self
    }

    fn do_move_no_checks(&mut self, move_action: &MoveAction) -> anyhow::Result<()> {
        ModifiableGame::do_move_no_checks(self, move_action)
    }
}

trait GamePrivate<B: Board + SafetyChecks>: PlayableGame<B> + ModifiableGame<B> {
    // Only for moves from the move generator, not from unknown sources
    fn clone_and_move_with_checks(
        &self,
        move_action: &MoveAction,
        king_position: &Position,
        reset_hash: bool,
    ) -> Option<GameState<B>> {
        let mv = &move_action.mv;
        let is_king = mv.source == *king_position;
        let player = self.board().at(king_position).unwrap().player;

        // Before moving, check if it is a castling and it is valid
        if is_king && mv.source.file.abs_diff(mv.target.file) == 2 {
            let is_valid_castling_square = |direction: &Direction| {
                only_empty_and_safe(
                    self.board(),
                    self.try_move(&mv.source, direction).next(),
                    &player,
                )
                .is_some()
            };
            let can_castle = match mv.target.file {
                // Queenside
                2 => {
                    is_valid_castling_square(&dir!(0, -1)) && is_valid_castling_square(&dir!(0, -2))
                }
                // Kingside
                6 => is_valid_castling_square(&dir!(0, 1)) && is_valid_castling_square(&dir!(0, 2)),
                _ => panic!(
                    "invalid castling {} in:\n{}\n,game info is {}",
                    mv,
                    self.board(),
                    self.info()
                ),
            };
            let castling_is_safe = can_castle && !self.board().is_piece_unsafe(king_position);

            if !castling_is_safe {
                return None;
            }
        }

        // Move
        let new_game = self.clone_and_move(move_action, reset_hash).ok()?;

        // After moving, check if the king is in check

        let current_king_position = if is_king { &mv.target } else { king_position };

        if new_game
            .as_ref()
            .board()
            .is_piece_unsafe(current_king_position)
        {
            return None;
        }

        Some(new_game)
    }

    fn get_possible_moves_iter<'a>(
        &'a self,
        position: Position,
    ) -> impl Iterator<Item = Position> + 'a
    where
        B: 'a,
    {
        piece_into_iter(self.as_ref(), position)
    }

    fn get_possible_moves_no_checks(&self, position: Position) -> Vec<MoveAction> {
        let mut possible_moves: Vec<MoveAction> = Vec::new();

        let is_pawn = match self.board().at(&position) {
            Some(piece) => piece.piece == PieceType::Pawn,
            None => false,
        };
        let can_promote = is_pawn && position.rank == B::can_promote_rank(&self.player());

        for target in self.get_possible_moves_iter(position) {
            if can_promote {
                // I don't think there is an easy way to iterate through all the values of an enum :(
                possible_moves.push(mva!(position, target));
                possible_moves.push(MoveAction {
                    mv: mv!(position, target),
                    move_type: MoveActionType::Promotion(PromotionPieces::Knight),
                });
                possible_moves.push(MoveAction {
                    mv: mv!(position, target),
                    move_type: MoveActionType::Promotion(PromotionPieces::Bishop),
                });
                possible_moves.push(MoveAction {
                    mv: mv!(position, target),
                    move_type: MoveActionType::Promotion(PromotionPieces::Rook),
                });
                possible_moves.push(MoveAction {
                    mv: mv!(position, target),
                    move_type: MoveActionType::Promotion(PromotionPieces::Queen),
                });
            } else {
                possible_moves.push(MoveAction {
                    mv: mv!(position, target),
                    move_type: MoveActionType::Normal,
                });
            }
        }

        possible_moves
    }

    fn get_possible_moves_from_game(&self, position: Position) -> Vec<MoveAction> {
        let Some(Piece { piece, player: _ }) = self.board().at(&position) else {
            return vec![];
        };
        let is_king = piece == PieceType::King;

        let king_position = if is_king {
            position
        } else {
            self.board().find_king(&self.player())
        };

        self.get_possible_moves_no_checks(position)
            .iter()
            .filter(|mv| {
                self.clone_and_move_with_checks(mv, &king_position, true)
                    .is_some()
            })
            .copied()
            .collect()
    }

    fn move_branch_names(&self, moves: &Vec<&MoveAction>) -> Vec<String> {
        let mut new_game = self.as_ref().clone();

        let mut move_names = vec![];

        for mv in moves {
            move_names.push(Game::move_name(&new_game, mv).unwrap());

            assert!(Game::do_move(&mut new_game, mv).is_some());
        }

        move_names
    }

    // Negamax with alpha-beta pruning
    fn get_best_move_recursive_alpha_beta(
        &self,
        current_depth: u32,
        max_depth: u32,
        scores: SearchScores,
        history: &mut HashedHistory,
        stop_signal: &mut impl HasStopSignal,
        feedback: &mut impl SearchFeedback,
    ) -> SearchResult {
        let board = self.board();
        let player = self.player();

        let mut playable_pieces = self
            .board_iter()
            .only_player(player)
            .collect::<Vec<Position>>()
            .into_iter();

        let mut best_move: Option<Branch> = None;

        let mut searched_moves: u32 = 0;

        let king_position = self.board().find_king(&player);

        let mut local_alpha = scores.alpha;

        let is_leaf_node = current_depth == max_depth;
        let mut stopped = false;

        #[allow(clippy::while_let_on_iterator)]
        'main_loop: while let Some(player_piece_position) = playable_pieces.next() {
            let mut possible_moves = self
                .get_possible_moves_from_game(player_piece_position)
                .into_iter();
            while let Some(possible_move) = possible_moves.next() {
                if stop_signal.stop() {
                    let _ = writeln!(feedback, "Search stopped");
                    stopped = true;
                    break 'main_loop;
                }

                let mut cutoff = false;

                let mv = &possible_move.mv;
                let possible_position = &mv.target;
                searched_moves += 1;

                // Evaluate this move locally
                let local_score = match &board.at(possible_position) {
                    Some(piece) => Score::piece_value(piece.piece),
                    None => {
                        match possible_move.move_type {
                            MoveActionType::Promotion(promotion_piece) => {
                                // Promotion
                                Score::piece_value(promotion_piece.into())
                            }
                            _ => Score::from(0),
                        }
                    }
                };

                let Some(mut recursive_game) =
                    self.clone_and_move_with_checks(&possible_move, &king_position, false)
                else {
                    continue;
                };

                // Threefold repetition
                let hash = recursive_game.hash();
                history.push(possible_move, hash);
                let repetition_count = history.count(&hash);
                let threefold_repetition = repetition_count >= 3;

                let mut branch = Branch {
                    moves: vec![WeightedMove {
                        mv: possible_move,
                        score: local_score,
                    }],
                    // Negamax: negate score from previous move
                    score: local_score - scores.parent,
                    searched: 0,
                    result: None,
                };

                feedback.update(current_depth, searched_moves, branch.score.into());

                #[cfg(feature = "verbose-search")]
                feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Child(
                    crate::eval::feedback::SearchMove {
                        mv: Some(possible_move),
                        info: format!(
                            "{}: {} {:+} α: {}, β: {}",
                            player, mv, branch.score, local_alpha, scores.beta
                        ),
                    },
                ));

                // Recursion
                if threefold_repetition {
                    // Enforce draw
                    branch.score = Score::stalemate();
                    branch.result = Some(GameResult::Draw);
                } else if !is_leaf_node {
                    let mut search_result = recursive_game.get_best_move_recursive_alpha_beta(
                        current_depth + 1,
                        max_depth,
                        // beta becomes the alpha of the other player, and viceversa
                        SearchScores {
                            parent: branch.score,
                            alpha: -scores.beta,
                            beta: -local_alpha,
                        },
                        history,
                        stop_signal,
                        feedback,
                    );

                    let next_moves_opt = &mut search_result.branch;
                    stopped = search_result.stopped;

                    let is_check_mate = if next_moves_opt.is_none() {
                        // check or stale mate?
                        let enemy_player_king_position =
                            recursive_game.as_ref().board().find_king(&!player);

                        recursive_game
                            .as_ref()
                            .board()
                            .is_piece_unsafe(&enemy_player_king_position)
                    } else {
                        false
                    };

                    #[cfg(feature = "verbose-search")]
                    feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Info(format!(
                        "best child: {}",
                        next_moves_opt
                            .as_ref()
                            .map_or("<mate>".to_string(), |sub_branch| {
                                format!("{}", sub_branch)
                            })
                    )));

                    if let Some(next_moves) = next_moves_opt {
                        branch.moves.append(&mut next_moves.moves);
                        branch.score = -next_moves.score; // notice the score of the child branch is negated
                        branch.searched = next_moves.searched;
                        branch.result = next_moves.result;
                    } else if is_check_mate {
                        branch.score = branch.score + Score::piece_value(PieceType::King);
                        branch.result = Some(GameResult::Win(player));
                    } else {
                        // Stalemate
                        branch.score = Score::stalemate();
                        branch.result = Some(GameResult::Draw);
                    }

                    searched_moves += branch.searched;
                }

                history.pop().unwrap();

                match &best_move {
                    Some(current_best_move) => {
                        if branch.score > current_best_move.score
                            || (branch.score == current_best_move.score
                                && branch.moves.len() < current_best_move.moves.len())
                        {
                            #[cfg(feature = "verbose-search")]
                            feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Info(
                                format!("new best move: {} > {}", branch, current_best_move),
                            ));

                            best_move = Some(branch);
                        }
                    }
                    None => {
                        #[cfg(feature = "verbose-search")]
                        feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Info(
                            format!("new best move: {}", branch),
                        ));

                        best_move = Some(branch);
                    }
                };

                if let Some(best_move_branch) = best_move.as_ref() {
                    let best_move_score = best_move_branch.score;

                    // If the best move is a mate in more than one, don't cutoff the search yet,
                    // because there's a chance that the remaining moves are mates in less
                    let is_mate_in_more_than_one = best_move_score
                        >= Score::piece_value(PieceType::King)
                        && best_move_branch.moves.len() > 1;

                    if best_move_score >= scores.beta {
                        if !is_mate_in_more_than_one {
                            // Fail hard beta cutoff

                            #[cfg(feature = "verbose-search")]
                            feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Info(
                                format!("β cutoff: {} >= {}", best_move_score, scores.beta,),
                            ));

                            cutoff = true;
                        } else {
                            #[cfg(feature = "verbose-search")]
                            feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Info(
                                format!(
                                    "Not cutting off because mate in more than one: {}",
                                    best_move_score
                                ),
                            ));
                        }
                    }

                    // This will be the beta of the next recursion
                    local_alpha = best_move_score;
                }

                if stopped || cutoff {
                    #[cfg(feature = "verbose-search")]
                    {
                        feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Info(
                            format!(
                                "Skipping remaining moves for this piece: [{}]",
                                possible_moves
                                    .map(|mv| format!("{}", mv.mv))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                        ));
                        feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Info(
                            format!(
                                "Skipping remaining pieces: [{}]",
                                playable_pieces
                                    .map(|position| format!("{}", position))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                        ));
                        feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Return);
                    }

                    break 'main_loop;
                }

                #[cfg(feature = "verbose-search")]
                feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Return);
            } // possible moves loop
        } // main loop

        if let Some(best_move) = best_move.as_mut() {
            best_move.searched = searched_moves;
        }

        SearchResult {
            branch: best_move,
            stopped,
        }
    }

    fn get_best_move_shallow(&self) -> Option<Branch> {
        self.get_best_move_recursive_alpha_beta(
            0,
            0,
            SearchScores::default(),
            &mut HashedHistory::default(),
            &mut (),
            &mut SilentSearchFeedback::default(),
        )
        .branch
    }

    fn get_possible_captures_of_position(&self, position: &Position) -> Vec<Position> {
        let mut captures: Vec<Position> = Vec::new();

        if let Some(square) = self.board().at(position) {
            for possible_position in self.get_possible_moves_iter(*position) {
                let is_capture = self.board().at(&possible_position).is_some();

                if is_capture {
                    captures.push(possible_position);
                } else if !is_capture
                    && square.piece == PieceType::Pawn
                    && position.file.abs_diff(position.file) != 0
                {
                    let passed_rank = usize::try_from(
                        i8::try_from(position.rank).unwrap()
                            - B::pawn_progress_direction(&square.player),
                    )
                    .unwrap();
                    captures.push(pos!(passed_rank, possible_position.file));
                }
            }
        }

        captures
    }
}

#[allow(private_bounds)]
pub trait Game<B: Board + SafetyChecks>: GamePrivate<B> {
    fn get_possible_moves(&self, position: Position) -> Vec<MoveAction> {
        let Some(Piece { piece: _, player }) = self.board().at(&position) else {
            return vec![];
        };

        let mut game = self.as_ref().clone_unhashed();

        // Play as the color of the position
        game.update_player(player);

        game.get_possible_moves_from_game(position)
    }

    fn get_possible_targets(&self, position: Position) -> Vec<Position> {
        let mut targets: Vec<Position> = self
            .get_possible_moves(position)
            .iter()
            .map(|&mv| mv.mv.target)
            .collect();
        targets.dedup();
        targets
    }

    fn get_all_possible_moves(&self) -> Vec<MoveAction> {
        let mut moves: Vec<MoveAction> = vec![];

        for piece_position in self.board_iter().only_player(self.player()) {
            moves.extend(self.get_possible_moves_from_game(piece_position));
        }

        moves
    }

    fn move_name(&self, move_action: &MoveAction) -> Result<String> {
        let board = self.board();
        let player = &self.player();
        let mv = &move_action.mv;
        let mut name = String::new();
        let Some(src_piece) = board.at(&mv.source) else {
            bail!("No piece at {}", mv.source);
        };

        let is_castling =
            src_piece.piece == PieceType::King && mv.source.file.abs_diff(mv.target.file) == 2;

        if is_castling {
            // Castling doesn't need piece or position
            match mv.target.file {
                2 => name.push_str("O-O-O"),
                6 => name.push_str("O-O"),
                _ => panic!("invalid castling {} in:\n{}", mv, board),
            }
        } else {
            let piece_char = |piece: &PieceType| match piece {
                PieceType::Knight => Some('N'),
                PieceType::Bishop => Some('B'),
                PieceType::Rook => Some('R'),
                PieceType::Queen => Some('Q'),
                PieceType::King => Some('K'),
                _ => None,
            };
            let tgt_piece_opt = board.at(&mv.target);
            let pieces_iter = self.board_iter().only_player(*player);

            if let Some(piece_char_value) = piece_char(&src_piece.piece) {
                name.push(piece_char_value);
            }

            let is_pawn = src_piece.piece == PieceType::Pawn;

            let is_en_passant =
                is_pawn && mv.source.file != mv.target.file && board.at(&mv.target).is_none();

            let mut ambiguous_piece_exists = false;
            let mut piece_in_same_file = false;
            let mut piece_in_same_rank = false;

            for player_piece_position in pieces_iter {
                if player_piece_position == mv.source {
                    continue;
                }

                if let Some(player_piece) = board.at(&player_piece_position) {
                    if player_piece.piece != src_piece.piece {
                        continue;
                    }
                }

                if self
                    .get_possible_moves_iter(player_piece_position)
                    .any(|possible_position| possible_position == mv.target)
                {
                    ambiguous_piece_exists = true;
                    if player_piece_position.rank == mv.source.rank {
                        piece_in_same_rank = true;
                    } else if player_piece_position.file == mv.source.file {
                        piece_in_same_file = true;
                    }
                }
            }

            let is_capture = tgt_piece_opt.is_some() || is_en_passant;

            let source_suffix = format!("{}", mv.source);
            let source_rank = source_suffix.chars().nth(1).unwrap();
            let source_file = source_suffix.chars().nth(0).unwrap();

            if is_en_passant || (is_pawn && is_capture) {
                name.push(source_file);
            } else if piece_in_same_file && piece_in_same_rank {
                // Same type of pieces in same rank and file: file and rank suffix
                name.push_str(source_suffix.as_str());
            } else if piece_in_same_rank {
                // Same type of pieces in same rank but different file: file suffix
                name.push(source_file);
            } else if piece_in_same_file {
                // Same type of pieces in same file but different rank: rank suffix
                name.push(source_rank);
            } else if ambiguous_piece_exists {
                // Another piece not in the same rank or file: file suffix
                name.push(source_file);
            }

            if is_capture {
                name.push('x');
            }

            name.push_str(format!("{}", mv.target).as_str());

            // Is promotion?
            if is_pawn && mv.target.rank == B::promotion_rank(player) {
                let MoveActionType::Promotion(promotion_piece) = move_action.move_type else {
                    bail!("Promotion piece not specified in {}", move_action.mv);
                };
                name.push('=');
                name.push(piece_char(&promotion_piece.into()).unwrap());
            }
        }

        // Is check?
        let mut new_game = self.as_ref().clone_unhashed();

        if Game::do_move(&mut new_game, move_action).is_some() {
            let enemy_king_position = new_game.board().find_king(&!*player);
            let causes_check = new_game.board().is_piece_unsafe(&enemy_king_position);
            if causes_check {
                let is_checkmate = new_game.get_best_move_shallow().is_none();

                name.push(if is_checkmate { '#' } else { '+' });
            }

            Ok(name)
        } else {
            bail!("Invalid move {}", move_action.mv);
        }
    }

    fn get_best_move_recursive(
        &self,
        history: &GameHistory,
        search_depth: u32,
        stop_signal: &mut impl HasStopSignal,
        feedback: &mut impl SearchFeedback,
    ) -> Option<Branch> {
        let mut hashed_history = HashedHistory::from(history).ok()?;

        hashed_history.reserve(search_depth as usize);

        #[cfg(feature = "verbose-search")]
        feedback.search_node(crate::eval::feedback::SearchNodeFeedback::Fen(
            self.as_ref().to_fen().to_string(),
        ));

        self.get_best_move_recursive_alpha_beta(
            0,
            search_depth,
            SearchScores::default(),
            &mut hashed_history,
            stop_signal,
            feedback,
        )
        .branch
    }

    fn get_possible_captures(&self) -> BoardCaptures {
        let mut board_captures: BoardCaptures = Default::default();

        for source_position in self.board_iter() {
            for capture in self.get_possible_captures_of_position(&source_position) {
                board_captures[capture.rank][capture.file].push(source_position);
            }
        }

        board_captures
    }

    fn get_best_move_with_logger(
        &self,
        history: &GameHistory,
        search_depth: u32,
        stop_signal: &mut impl HasStopSignal,
        feedback: &mut impl SearchFeedback,
    ) -> GameMove {
        let player = self.player();
        let start_time = Instant::now();

        let best_branch =
            self.get_best_move_recursive(history, search_depth, stop_signal, feedback);

        let duration = (Instant::now() - start_time).as_secs_f64();

        if best_branch.is_none() {
            // check or stale mate?
            let king_position = self.board().find_king(&player);
            let is_check_mate = self.board().is_piece_unsafe(&king_position);

            log!(feedback, "  ({:.2} s.) ", duration);
            let enemy_player = !player;
            if is_check_mate {
                log!(feedback, "Checkmate, {} wins", enemy_player);
            } else {
                log!(feedback, "Stalemate caused by {}", enemy_player);
            }
            return if is_check_mate {
                GameMove::Mate(MateType::Checkmate)
            } else {
                GameMove::Mate(MateType::Stalemate)
            };
        }

        let total_score = best_branch
            .as_ref()
            .map(|weighted_move| weighted_move.score)
            .unwrap();

        let total_moves = best_branch
            .as_ref()
            .map(|weighted_move| weighted_move.searched)
            .unwrap();

        let branch_moves = best_branch
            .as_ref()
            .unwrap()
            .moves
            .iter()
            .map(|mv| &mv.mv)
            .collect::<Vec<&MoveAction>>();

        log!(
            feedback,
            "  ({:.2} s., {:.0} mps) Best branch {:+} after {}: {}",
            duration,
            f64::from(total_moves) / duration,
            total_score,
            total_moves,
            std::iter::zip(
                self.move_branch_names(&branch_moves),
                &best_branch.as_ref().unwrap().moves
            )
            .map(|(move_name, move_info)| format!("{}{:+}", move_name, move_info.score))
            .collect::<Vec<String>>()
            .join(" ")
        );

        GameMove::Normal(**branch_moves.first().unwrap())
    }

    fn get_best_move(&self, history: &GameHistory, search_depth: u32) -> GameMove {
        self.get_best_move_with_logger(history, search_depth, &mut (), &mut StdoutFeedback)
    }

    fn is_mate(&self) -> Option<MateType> {
        if self.get_best_move_shallow().is_none() {
            let board = self.board();
            let king_position = board.find_king(&self.player());
            return if board.is_piece_unsafe(&king_position) {
                Some(MateType::Checkmate)
            } else {
                Some(MateType::Stalemate)
            };
        }

        None
    }

    fn do_move(&mut self, mv: &MoveAction) -> Option<Vec<Piece>> {
        let enemy_player = !self.player();
        let mut enemy_army: HashMap<PieceType, u32> = HashMap::new();

        for piece in self
            .board_iter()
            .only_player(enemy_player)
            .map(|position| self.board().at(&position).unwrap().piece)
        {
            match enemy_army.get_mut(&piece) {
                Some(piece_count) => {
                    *piece_count += 1;
                }
                None => {
                    enemy_army.insert(piece, 1);
                }
            }
        }

        let result = PlayableGame::do_move_with_checks(self.as_mut(), mv);

        if result {
            for piece in self
                .board_iter()
                .only_player(enemy_player)
                .map(|position| self.board().at(&position).unwrap().piece)
            {
                match enemy_army.get_mut(&piece) {
                    Some(piece_count) => {
                        *piece_count -= 1;
                        if *piece_count == 0 {
                            assert!(enemy_army.remove(&piece).is_some());
                        }
                    }
                    None => {
                        // A pawn has been promoted to something else (promotion is mandatory)
                        assert_ne!(piece, PieceType::Pawn);
                        assert!(enemy_army.remove(&PieceType::Pawn).is_some());
                    }
                }
            }
            Some(
                enemy_army
                    .keys()
                    .map(|piece| Piece {
                        piece: *piece,
                        player: enemy_player,
                    })
                    .collect(),
            )
        } else {
            None
        }
    }
}

impl<B: Board + SafetyChecks> GamePrivate<B> for GameState<B> {}

impl<B: Board + SafetyChecks> Game<B> for GameState<B> {}
