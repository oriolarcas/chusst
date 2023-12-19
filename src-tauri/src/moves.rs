mod bitboards;
mod check;
mod conditions;
mod feedback;
mod iter;
mod play;

use crate::board::{Board, Piece, PieceType, Player, Position, Ranks};
use crate::game::{Game, GameInfo, Move, MoveInfo};
use crate::moves::bitboards::Bitboards;
use crate::moves::check::{find_player_king, only_empty_and_safe, piece_is_unsafe};
pub use crate::moves::feedback::{EngineFeedback, EngineFeedbackMessage, SilentSearchFeedback};
use crate::moves::feedback::{PeriodicalSearchFeedback, SearchFeedback, StdoutFeedback};
use crate::moves::play::SearchableGame;
use crate::{mv, pos};
use conditions::{enemy, Direction};
use iter::{dir, piece_into_iter, player_pieces_iter, BoardIter, PlayerPiecesIter};

use core::panic;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

use self::check::piece_is_unsafe_with_bitboards;
use self::conditions::try_move;
use self::play::{PlayableGame, ReversableGame};

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

impl Score {
    pub const MAX: Self = Self(i32::MAX);
    pub const MIN: Self = Self(-i32::MAX); // -i32::MIN > i32::MAX
}

impl From<i32> for Score {
    fn from(value: i32) -> Self {
        Score(value)
    }
}

impl Into<i32> for Score {
    fn into(self) -> i32 {
        self.0
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
    Normal(Move),
    Mate(MateType),
}

#[derive(PartialEq)]
pub struct WeightedMove {
    pub mv: Move,
    pub score: Score,
}

#[derive(Default)]
pub struct Branch {
    pub moves: Vec<WeightedMove>,
    pub score: Score,
    pub searched: u32,
}

struct SearchResult {
    branch: Option<Branch>,
    stopped: bool,
}

fn move_with_checks(game: &mut SearchableGame, mv: &Move, king_position: &Position) -> bool {
    let is_king = mv.source == *king_position;
    let player = game.as_ref().board.square(king_position).unwrap().player;

    // Before moving, check if it is a castling and it is valid
    if is_king && mv.source.file.abs_diff(mv.target.file) == 2 {
        let bitboards = Bitboards::from(&game.as_ref().board);
        let player_bitboards = bitboards.by_player(&player);
        let enemy_bitboards = bitboards.by_player(&enemy(&player));

        let is_valid_castling_square = |direction: &Direction| {
            only_empty_and_safe(
                &game.as_ref().board,
                try_move(&mv.source, &direction),
                &player,
                &player_bitboards,
                &enemy_bitboards,
            )
            .is_some()
        };
        let can_castle = match mv.target.file {
            // Queenside
            2 => is_valid_castling_square(&dir!(0, -1)) && is_valid_castling_square(&dir!(0, -2)),
            // Kingside
            6 => is_valid_castling_square(&dir!(0, 1)) && is_valid_castling_square(&dir!(0, 2)),
            _ => panic!(
                "invalid castling {} in:\n{}\n,game info is {}",
                mv,
                game.as_ref().board,
                game.as_ref().info
            ),
        };
        let castling_is_safe = can_castle
            && !piece_is_unsafe_with_bitboards(
                &game.as_ref().board,
                king_position,
                &player_bitboards,
                &enemy_bitboards,
            );

        if !castling_is_safe {
            return false;
        }
    }

    // Move
    game.do_move_no_checks(mv);

    // After moving, check if the king is in check

    let current_king_position = if is_king { &mv.target } else { king_position };

    // let bitboards = Bitboards::from(&game.as_ref().board);
    // let player_bitboards = bitboards.by_player(&player);
    // let enemy_bitboards = bitboards.by_player(&enemy(&player));
    // !piece_is_unsafe_with_bitboards(
    //     &game.as_ref().board,
    //     &current_king_position,
    //     &player_bitboards,
    //     &enemy_bitboards,
    // )

    !piece_is_unsafe(&game.as_ref().board, &current_king_position)
}

fn get_possible_moves_iter<'a>(
    board: &'a Board,
    last_move: &'a Option<MoveInfo>,
    game_info: &'a GameInfo,
    position: Position,
) -> impl Iterator<Item = Position> + 'a {
    piece_into_iter(board, last_move, game_info, position)
}

fn get_possible_moves_no_checks(
    board: &Board,
    last_move: &Option<MoveInfo>,
    game_info: &GameInfo,
    position: Position,
) -> Vec<Position> {
    get_possible_moves_iter(board, last_move, game_info, position).collect::<Vec<Position>>()
}

pub fn get_possible_moves(
    board: &Board,
    last_move: &Option<MoveInfo>,
    game_info: &GameInfo,
    position: Position,
) -> Vec<Position> {
    let square = board.square(&position);
    if square.is_none() {
        return vec![];
    }

    let player = square.unwrap().player;
    let is_king = square.unwrap().piece == PieceType::King;

    let king_position = if is_king {
        position
    } else {
        find_player_king(&board, &player)
    };

    let mut game = Game {
        board: *board,
        player,
        last_move: *last_move,
        info: *game_info,
    };
    get_possible_moves_no_checks(board, last_move, game_info, position)
        .iter()
        .filter(|possible_position| {
            let mv = mv!(position, **possible_position);

            let mut rev_game = SearchableGame::from_game(&mut game);

            move_with_checks(&mut rev_game, &mv, &king_position)
        })
        .copied()
        .collect()
}

pub fn move_name(
    board: &Board,
    last_move: &Option<MoveInfo>,
    game_info: &GameInfo,
    player: &Player,
    mv: &Move,
) -> Option<String> {
    let mut name = String::new();
    let src_piece_opt = board.square(&mv.source);
    if src_piece_opt.is_none() {
        return None;
    }
    let src_piece = src_piece_opt.unwrap();

    let is_castling =
        src_piece.piece == PieceType::King && mv.source.file.abs_diff(mv.target.file) == 2;

    if is_castling {
        // Castling doesn't need piece or position
        match mv.target.file {
            2 => name.push_str("0-0-0"),
            6 => name.push_str("0-0"),
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
        let tgt_piece_opt = board.square(&mv.target);
        let pieces_iter = player_pieces_iter!(board: board, player: player);

        match piece_char(&src_piece.piece) {
            Some(piece_char_value) => name.push(piece_char_value),
            None => (),
        }

        let is_pawn = src_piece.piece == PieceType::Pawn;

        let is_en_passant =
            is_pawn && mv.source.file != mv.target.file && board.square(&mv.target).is_none();

        let mut piece_in_same_file = false;
        let mut piece_in_same_rank = false;

        for player_piece_position in pieces_iter {
            if player_piece_position == mv.source {
                continue;
            }

            match board.square(&player_piece_position) {
                Some(player_piece) => {
                    if player_piece.piece != src_piece.piece {
                        continue;
                    }
                }
                None => {}
            }

            if get_possible_moves_iter(board, last_move, game_info, player_piece_position)
                .find(|possible_position| *possible_position == mv.target)
                .is_some()
            {
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
        }

        if is_capture {
            name.push('x');
        }

        name.push_str(format!("{}", mv.target).as_str());

        // Is promotion?
        if is_pawn && mv.target.rank == Board::promotion_rank(player) {
            name.push('=');
            name.push(piece_char(&PieceType::Queen).unwrap());
        }
    }

    // Is check?
    let mut game = Game {
        board: *board,
        player: src_piece.player,
        last_move: *last_move,
        info: *game_info,
    };

    if do_move(&mut game, mv).is_some() {
        let enemy_king_position = find_player_king(&game.board, &enemy(player));
        let causes_check =
            get_possible_captures_of_position(&game.board, last_move, game_info, &mv.target)
                .iter()
                .find(|position| **position == enemy_king_position)
                .is_some();
        if causes_check {
            let is_checkmate = get_best_move_shallow(&mut game).is_none();

            name.push(if is_checkmate { '#' } else { '+' });
        }
    }

    Some(name)
}

pub fn move_branch_names(
    board: &Board,
    player: &Player,
    game_info: &GameInfo,
    moves: &Vec<&Move>,
) -> Vec<String> {
    let mut game = Game {
        board: *board,
        player: *player,
        last_move: None,
        info: *game_info,
    };

    let mut move_names = vec![];

    for mv in moves {
        move_names
            .push(move_name(&game.board, &game.last_move, &game.info, &game.player, &mv).unwrap());

        assert!(do_move(&mut game, *mv).is_some());
    }

    move_names
}

// Value in centipawns
fn get_piece_value(piece: PieceType) -> Score {
    match piece {
        PieceType::Pawn => Score::from(100),
        PieceType::Knight => Score::from(300),
        PieceType::Bishop => Score::from(300),
        PieceType::Rook => Score::from(500),
        PieceType::Queen => Score::from(900),
        PieceType::King => Score::MAX,
    }
}

// Negamax with alpha-beta pruning
fn get_best_move_recursive_alpha_beta(
    game: &mut Game,
    current_depth: u32,
    max_depth: u32,
    alpha: Score,
    beta: Score,
    parent_score: Score,
    stop_signal: &mut impl HasStopSignal,
    feedback: &mut impl SearchFeedback,
) -> SearchResult {
    let pieces_iter =
        player_pieces_iter!(board: &game.board, player: &game.player).collect::<Vec<Position>>();

    let mut best_move: Option<Branch> = None;

    let mut searched_moves: u32 = 0;

    let player = game.player;
    let king_position = find_player_king(&game.board, &game.player);

    let mut local_alpha = alpha;

    let is_leaf_node = current_depth == max_depth;
    let mut stopped = false;

    #[cfg(feature = "verbose-search")]
    let indent = |depth: u32| {
        std::iter::repeat("  ")
            .take(usize::try_from(depth).unwrap())
            .collect::<String>()
    };

    'main_loop: for player_piece_position in pieces_iter {
        let current_piece = &game.board.square(&player_piece_position).unwrap().piece;

        for possible_position in get_possible_moves_no_checks(
            &game.board,
            &game.last_move,
            &game.info,
            player_piece_position,
        ) {
            if stop_signal.stop() {
                let _ = writeln!(feedback, "Search stopped");
                break 'main_loop;
            }

            searched_moves += 1;

            // Evaluate this move locally
            let local_score = match &game.board.square(&possible_position) {
                Some(piece) => get_piece_value(piece.piece),
                None => {
                    if *current_piece == PieceType::Pawn
                        && possible_position.rank == Board::promotion_rank(&player)
                    {
                        // Promotion
                        get_piece_value(PieceType::Queen)
                    } else {
                        Score::from(0)
                    }
                }
            };

            let mv = mv!(player_piece_position, possible_position);

            let mut rev_game = SearchableGame::from_game(game);

            // Check if the move is legal
            if !move_with_checks(&mut rev_game, &mv, &king_position) {
                continue;
            }

            let mut branch = Branch {
                moves: vec![WeightedMove {
                    mv: mv,
                    score: local_score,
                }],
                // Negamax: negate score from previous move
                score: local_score - parent_score,
                searched: 0,
            };

            feedback.update(current_depth, searched_moves, branch.score.into());

            #[cfg(feature = "verbose-search")]
            if !silent {
                println!(
                    "{}{} {} {:+} α: {}, β: {}{}",
                    indent(current_depth),
                    player,
                    mv,
                    branch.score,
                    local_alpha,
                    beta,
                    if !is_leaf_node { " {" } else { "" },
                );
            }

            // Recursion
            if !is_leaf_node {
                let mut search_result = get_best_move_recursive_alpha_beta(
                    rev_game.as_mut(),
                    current_depth + 1,
                    max_depth,
                    // beta becomes the alpha of the other player, and viceversa
                    -beta,
                    -local_alpha,
                    branch.score,
                    stop_signal,
                    feedback,
                );

                let next_moves_opt = &mut search_result.branch;
                stopped = search_result.stopped;

                let is_check_mate = if next_moves_opt.is_none() {
                    // check or stale mate?
                    let enemy_player_king_position =
                        find_player_king(&rev_game.as_ref().board, &enemy(&player));

                    piece_is_unsafe(&rev_game.as_ref().board, &enemy_player_king_position)
                } else {
                    false
                };

                #[cfg(feature = "verbose-search")]
                if !silent {
                    println!(
                        "{}Best child: {}",
                        indent(current_depth + 1),
                        next_moves_opt
                            .as_ref()
                            .map_or("<mate>".to_string(), |sub_branch| format!(
                                "{}{:+}",
                                sub_branch.moves.first().unwrap().mv,
                                sub_branch.score
                            ))
                    );
                }

                let is_stale_mate = next_moves_opt.is_none() && !is_check_mate;

                if is_check_mate {
                    branch.score = branch.score + get_piece_value(PieceType::King);
                } else if !is_stale_mate {
                    let next_moves = next_moves_opt.as_mut().unwrap();

                    branch.moves.append(&mut next_moves.moves);
                    branch.score = -next_moves.score; // notice the score of the next move is negated
                    branch.searched = next_moves.searched;
                }

                searched_moves += branch.searched;

                #[cfg(feature = "verbose-search")]
                if !silent {
                    println!("{}}}", indent(current_depth));
                }

                if branch.score >= beta && branch.score < Score::MAX {
                    // Fail hard beta cutoff

                    #[cfg(feature = "verbose-search")]
                    if !silent {
                        println!(
                            "{}β cutoff: {} >= {}",
                            indent(current_depth),
                            branch.score,
                            beta
                        );
                    }

                    if best_move.as_ref().is_none() {
                        best_move = Some(branch);
                    }

                    best_move.as_mut().unwrap().score = beta;

                    break 'main_loop;
                }
            }

            match &best_move {
                Some(current_best_move) => {
                    if branch.score > current_best_move.score
                        || (branch.score == current_best_move.score
                            && branch.moves.len() < current_best_move.moves.len())
                    {
                        best_move = Some(branch);
                    }
                }
                None => {
                    best_move = Some(branch);
                }
            }

            // This will be the beta for the next move
            local_alpha = best_move.as_ref().unwrap().score;

            if stopped {
                break 'main_loop;
            }
        }
    }

    match &mut best_move {
        Some(best_move) => {
            best_move.searched = searched_moves;
        }
        None => (),
    }

    SearchResult {
        branch: best_move,
        stopped,
    }
}

fn get_best_move_shallow(game: &mut Game) -> Option<Branch> {
    get_best_move_recursive_alpha_beta(
        game,
        0,
        0,
        Score::MIN,
        Score::MAX,
        Score::from(0),
        &mut (),
        &mut SilentSearchFeedback::default(),
    )
    .branch
}

pub fn get_best_move_recursive(
    game: &mut Game,
    search_depth: u32,
    stop_signal: &mut impl HasStopSignal,
    feedback: &mut impl SearchFeedback,
) -> Option<Branch> {
    get_best_move_recursive_alpha_beta(
        game,
        0,
        search_depth,
        Score::MIN,
        Score::MAX,
        Score::from(0),
        stop_signal,
        feedback,
    )
    .branch
}

fn get_possible_captures_of_position(
    board: &Board,
    last_move: &Option<MoveInfo>,
    game_info: &GameInfo,
    position: &Position,
) -> Vec<Position> {
    let mut captures: Vec<Position> = Vec::new();

    match board.square(position) {
        Some(square) => {
            for possible_position in get_possible_moves_iter(board, last_move, game_info, *position)
            {
                let is_capture = board.square(&possible_position).is_some();

                if is_capture {
                    captures.push(possible_position);
                } else if !is_capture
                    && square.piece == PieceType::Pawn
                    && position.file.abs_diff(position.file) != 0
                {
                    let passed_rank = usize::try_from(
                        i8::try_from(position.rank).unwrap()
                            - Board::pawn_progress_direction(&square.player),
                    )
                    .unwrap();
                    captures.push(pos!(passed_rank, possible_position.file));
                }
            }
        }
        None => (),
    }

    captures
}

pub fn get_possible_captures(
    board: &Board,
    last_move: &Option<MoveInfo>,
    game_info: &GameInfo,
) -> BoardCaptures {
    let board_iter: BoardIter = Default::default();
    let mut board_captures: BoardCaptures = Default::default();

    for source_position in board_iter.into_iter() {
        for capture in
            get_possible_captures_of_position(board, last_move, game_info, &source_position)
        {
            board_captures[capture.rank][capture.file].push(source_position);
        }
    }

    board_captures
}

pub fn get_best_move_with_logger(
    game: &mut Game,
    search_depth: u32,
    stop_signal: &mut impl HasStopSignal,
    engine_feedback: &mut impl EngineFeedback,
) -> GameMove {
    let player = game.player;
    let start_time = Instant::now();
    let mut feedback =
        PeriodicalSearchFeedback::new(std::time::Duration::from_millis(500), engine_feedback);
    let best_branch = get_best_move_recursive(game, search_depth, stop_signal, &mut feedback);
    let duration = (Instant::now() - start_time).as_secs_f64();

    if best_branch.is_none() {
        // check or stale mate?
        let king_position = find_player_king(&game.board, &player);
        let is_check_mate = piece_is_unsafe(&game.board, &king_position);

        log!(engine_feedback, "  ({:.2} s.) ", duration);
        let enemy_player = enemy(&player);
        if is_check_mate {
            log!(engine_feedback, "Checkmate, {} wins", enemy_player);
        } else {
            log!(engine_feedback, "Stalemate caused by {}", enemy_player);
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
        .collect::<Vec<&Move>>();

    log!(
        engine_feedback,
        "  ({:.2} s., {:.0} mps) Best branch {:+} after {}: {}",
        duration,
        f64::from(total_moves) / duration,
        total_score,
        total_moves,
        std::iter::zip(
            &best_branch.as_ref().unwrap().moves,
            move_branch_names(&game.board, &game.player, &game.info, &branch_moves)
        )
        .map(|(move_info, move_name)| format!("{}{:+}", move_name, move_info.score))
        .collect::<Vec<String>>()
        .join(" ")
    );

    GameMove::Normal(**branch_moves.first().unwrap())
}

pub fn get_best_move(game: &mut Game, search_depth: u32) -> GameMove {
    get_best_move_with_logger(game, search_depth, &mut (), &mut StdoutFeedback::default())
}

pub fn is_mate(
    board: &Board,
    player: &Player,
    last_move: &Option<MoveInfo>,
    game_info: &GameInfo,
) -> Option<MateType> {
    // Is check?
    let mut game = Game {
        board: *board,
        player: *player,
        last_move: *last_move,
        info: *game_info,
    };

    if get_best_move_shallow(&mut game).is_none() {
        let king_position = find_player_king(&board, &player);
        return if piece_is_unsafe(&board, &king_position) {
            Some(MateType::Checkmate)
        } else {
            Some(MateType::Stalemate)
        };
    }

    None
}

pub fn do_move(game: &mut Game, mv: &Move) -> Option<Vec<Piece>> {
    let enemy_player = enemy(&game.player);
    let mut enemy_army: HashMap<PieceType, u32> = HashMap::new();

    for piece in player_pieces_iter!(board: &game.board, player: &enemy_player)
        .map(|position| game.board.square(&position).unwrap().piece)
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

    let mut rev_game = <ReversableGame as PlayableGame>::from_game(game);
    let result = rev_game.do_move(mv);

    if result {
        for piece in player_pieces_iter!(board: &game.board, player: &enemy_player)
            .map(|position| game.board.square(&position).unwrap().piece)
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
                .into_iter()
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

#[cfg(test)]
mod tests {
    use super::play::{PlayableGame, ReversableGame};
    use super::{do_move, get_possible_moves, move_name, mv, piece_is_unsafe};
    use crate::board::{Board, Piece, PieceType, Player, Position};
    use crate::game::{Game, Move};
    use crate::{p, pos};

    struct PiecePosition {
        piece: Option<Piece>,
        position: Position,
    }

    macro_rules! pp {
        ($piece:ident @ $pos:ident) => {
            PiecePosition {
                piece: p!($piece),
                position: pos!($pos),
            }
        };
        ($pos:ident) => {
            PiecePosition {
                piece: None,
                position: pos!($pos),
            }
        };
    }

    struct TestBoard<'a> {
        board: Option<&'a str>,
        initial_moves: Vec<Move>,
        mv: Move,
        checks: Vec<PiecePosition>,
    }

    fn custom_board(board_opt: &Option<&str>) -> Board {
        match board_opt {
            Some(board_str) => {
                let mut board = Board::default();

                let mut rank = 8usize;
                for line in board_str.lines() {
                    match line.find("[") {
                        Some(position) => {
                            let mut file = 0usize;
                            rank -= 1;

                            for piece_char in line
                                .chars()
                                .skip(position)
                                .filter(|c| *c != '[' && *c != ']')
                            {
                                let piece = match piece_char {
                                    '♙' => p!(pw),
                                    '♘' => p!(nw),
                                    '♗' => p!(bw),
                                    '♖' => p!(rw),
                                    '♕' => p!(qw),
                                    '♔' => p!(kw),
                                    '♟' => p!(pb),
                                    '♞' => p!(nb),
                                    '♝' => p!(bb),
                                    '♜' => p!(rb),
                                    '♛' => p!(qb),
                                    '♚' => p!(kb),
                                    ' ' => p!(),
                                    _ => {
                                        panic!(
                                            "unexpected character '\\u{:x}' in board line: {}",
                                            piece_char as u32, line
                                        )
                                    }
                                };
                                board.update(&pos!(rank, file), piece);
                                file += 1;
                            }
                        }
                        None => continue,
                    }
                }
                board
            }
            None => Board::new(),
        }
    }

    #[test]
    fn move_reversable() {
        let test_boards = [
            // Advance pawn
            TestBoard {
                board: None,
                initial_moves: vec![],
                mv: mv!(e2 => e3),
                checks: vec![pp!(pw @ e3), pp!(e2)],
            },
            // Pass pawn
            TestBoard {
                board: None,
                initial_moves: vec![],
                mv: mv!(e2 => e4),
                checks: vec![pp!(pw @ e4), pp!(e2)],
            },
            // Pawn capturing
            TestBoard {
                board: None,
                initial_moves: vec![mv!(e2 => e4), mv!(d7 => d5)],
                mv: mv!(e4 => d5),
                checks: vec![pp!(pw @ d5), pp!(e4)],
            },
            // Pawn capturing en passant
            TestBoard {
                board: None,
                initial_moves: vec![mv!(e2 => e4), mv!(a7 => a6), mv!(e4 => e5), mv!(d7 => d5)],
                mv: mv!(e5 => d6),
                checks: vec![pp!(pw @ d6), pp!(e5), pp!(d5)],
            },
            // Pawn promotion
            TestBoard {
                board: None,
                initial_moves: vec![
                    mv!(h2 => h4),
                    mv!(g7 => g6),
                    mv!(h4 => h5),
                    mv!(a7 => a6),
                    mv!(h5 => g6),
                    mv!(a6 => a5),
                    mv!(g6 => g7),
                    mv!(a5 => a4),
                ],
                mv: mv!(g7 => h8),
                checks: vec![pp!(qw @ h8)],
            },
            // Kingside castling
            TestBoard {
                board: None,
                initial_moves: vec![
                    mv!(e2 => e3),
                    mv!(a7 => a6),
                    mv!(f1 => e2),
                    mv!(b7 => b6),
                    mv!(g1 => h3),
                    mv!(c7 => c6),
                ],
                mv: mv!(e1 => g1),
                checks: vec![pp!(kw @ g1), pp!(rw @ f1)],
            },
            // Queenside castling
            TestBoard {
                board: None,
                initial_moves: vec![
                    mv!(d2 => d4),
                    mv!(a7 => a6),
                    mv!(d1 => d3),
                    mv!(b7 => b6),
                    mv!(c1 => d2),
                    mv!(c7 => c6),
                    mv!(b1 => c3),
                    mv!(d7 => d6),
                ],
                mv: mv!(e1 => c1),
                checks: vec![pp!(kw @ c1), pp!(rw @ d1)],
            },
        ];

        for test_board in &test_boards {
            // Prepare board
            let mut game = Game {
                board: custom_board(&test_board.board),
                player: Player::White,
                last_move: None,
                info: Default::default(),
            };

            // Do setup moves
            for mv in &test_board.initial_moves {
                assert!(
                    do_move(&mut game, &mv).is_some(),
                    "move {} failed:\n{}",
                    mv,
                    game.board
                );
            }

            let original_board = game.board.clone();

            let mut rev_game = <ReversableGame as PlayableGame>::from_game(&mut game);

            // Do move
            assert!(
                rev_game.do_move(&test_board.mv),
                "failed to make legal move {} in:\n{}",
                test_board.mv,
                game.board
            );

            for check in &test_board.checks {
                assert_eq!(
                    rev_game.as_ref().board.square(&check.position),
                    check.piece,
                    "expected {} in {}, found {}:\n{}",
                    check
                        .piece
                        .map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                    check.position,
                    rev_game
                        .as_ref()
                        .board
                        .square(&check.position)
                        .map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                    rev_game.as_ref().board,
                );
            }

            rev_game.undo();

            assert_eq!(
                game.board, original_board,
                "after move {},\nmodified board:\n{}\noriginal board:\n{}",
                test_board.mv, game.board, original_board
            );
        }
    }

    #[test]
    fn check_mate() {
        // White: ♙ ♘ ♗ ♖ ♕ ♔
        // Black: ♟ ♞ ♝ ♜ ♛ ♚
        let test_boards = [
            TestBoard {
                board: Some(
                    "  a  b  c  d  e  f  g  h \n\
                    8 [ ][ ][ ][ ][ ][ ][ ][♚]\n\
                    7 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    6 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    5 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    4 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    3 [ ][♛][ ][ ][ ][ ][ ][ ]\n\
                    2 [ ][ ][♛][ ][ ][ ][ ][ ]\n\
                    1 [♔][ ][ ][ ][ ][ ][ ][ ]",
                ),
                initial_moves: vec![],
                mv: mv!(b3 => b2),
                checks: vec![],
            },
            TestBoard {
                board: Some(
                    "  a  b  c  d  e  f  g  h \n\
                    8 [♜][♜][ ][ ][ ][ ][ ][♚]\n\
                    7 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    6 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    5 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    4 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    3 [ ][♟][ ][ ][ ][ ][ ][ ]\n\
                    2 [♟][ ][ ][ ][ ][ ][ ][ ]\n\
                    1 [♔][ ][ ][ ][ ][ ][ ][ ]",
                ),
                initial_moves: vec![],
                mv: mv!(b3 => b2),
                checks: vec![],
            },
            TestBoard {
                board: Some(
                    "  a  b  c  d  e  f  g  h \n\
                    8 [ ][♜][ ][ ][ ][ ][ ][♚]\n\
                    7 [ ][♜][ ][ ][ ][ ][ ][ ]\n\
                    6 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    5 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    4 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    3 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    2 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    1 [♔][ ][ ][ ][ ][ ][ ][ ]",
                ),
                initial_moves: vec![],
                mv: mv!(b8 => a8),
                checks: vec![],
            },
            TestBoard {
                board: Some(
                    "  a  b  c  d  e  f  g  h \n\
                    8 [ ][ ][ ][ ][ ][♝][♝][♚]\n\
                    7 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    6 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    5 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    4 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    3 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    2 [ ][ ][♟][ ][ ][ ][ ][ ]\n\
                    1 [♔][ ][ ][ ][ ][ ][ ][ ]",
                ),
                initial_moves: vec![],
                mv: mv!(f8 => g7),
                checks: vec![],
            },
            TestBoard {
                board: Some(
                    "  a  b  c  d  e  f  g  h \n\
                    8 [ ][ ][ ][ ][ ][ ][ ][♚]\n\
                    7 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    6 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    5 [♞][ ][ ][ ][ ][ ][ ][ ]\n\
                    4 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                    3 [ ][ ][♞][ ][ ][ ][ ][ ]\n\
                    2 [ ][ ][ ][ ][ ][ ][ ][♜]\n\
                    1 [♔][ ][ ][ ][ ][ ][ ][ ]",
                ),
                initial_moves: vec![],
                mv: mv!(a5 => b3),
                checks: vec![],
            },
        ];

        for test_board in test_boards {
            // Prepare board
            let mut game = Game {
                board: custom_board(&test_board.board),
                player: Player::Black,
                last_move: None,
                info: Default::default(),
            };

            game.info.disable_castle_kingside(&Player::White);
            game.info.disable_castle_kingside(&Player::Black);
            game.info.disable_castle_queenside(&Player::White);
            game.info.disable_castle_queenside(&Player::Black);

            // Do setup moves
            for mv in &test_board.initial_moves {
                assert!(
                    do_move(&mut game, &mv).is_some(),
                    "move {} failed:\n{}",
                    mv,
                    game.board
                );
            }

            let name = move_name(
                &game.board,
                &game.last_move,
                &game.info,
                &game.player,
                &test_board.mv,
            )
            .unwrap();
            assert!(
                name.ends_with("#"),
                "notation {} for move {} doesn't show checkmate sign # in:\n{}",
                name,
                test_board.mv,
                game.board
            );

            // Do move
            let mut rev_game = <ReversableGame as PlayableGame>::from_game(&mut game);

            assert!(
                rev_game.do_move(&test_board.mv),
                "invalid move {}:\n{}",
                test_board.mv,
                rev_game.as_ref().board
            );

            let possible_moves = get_possible_moves(&game.board, &None, &game.info, pos!(a1));
            let in_check = piece_is_unsafe(&game.board, &pos!(a1));
            assert!(in_check, "king should be in check:\n{}", game.board);
            assert!(
                possible_moves.is_empty(),
                "unexpected possible move {} in check mate:\n{}",
                mv!(pos!(a1), *possible_moves.first().unwrap()),
                game.board
            );
        }
    }

    #[test]
    fn fen_parsing() {
        let start_pos_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let parsed_game = Game::try_from_fen(
            start_pos_fen
                .split_ascii_whitespace()
                .collect::<Vec<&str>>()
                .as_slice(),
        );
        assert!(parsed_game.is_some(), "Failed to parse FEN string");
        let game = parsed_game.unwrap();
        assert_eq!(game, Game::new(), "\n{}", game.board);
    }

    // Template to quickly test a specific board/move
    #[test]
    #[ignore]
    fn quick_test() {
        // White: ♙ ♘ ♗ ♖ ♕ ♔
        // Black: ♟ ♞ ♝ ♜ ♛ ♚
        let test_boards = [TestBoard {
            board: Some(
                "  a  b  c  d  e  f  g  h \n\
                    8 [♜][♞][ ][♛][♚][♝][♞][ ]\n\
                    7 [ ][♝][♟][♟][♟][♟][♟][♜]\n\
                    6 [ ][♟][ ][ ][ ][ ][ ][ ]\n\
                    5 [♟][ ][ ][ ][ ][ ][ ][ ]\n\
                    4 [♙][ ][ ][♙][♙][ ][ ][♙]\n\
                    3 [ ][♙][♘][ ][♗][♘][♙][ ]\n\
                    2 [ ][ ][♙][♕][ ][ ][ ][ ]\n\
                    1 [♖][ ][ ][ ][♔][ ][ ][♖]",
            ),
            initial_moves: vec![],
            mv: mv!(e1 => c1),
            checks: vec![],
        }];

        for test_board in test_boards {
            // Prepare board
            let mut game = Game {
                board: custom_board(&test_board.board),
                player: Player::White,
                last_move: None,
                info: Default::default(),
            };

            game.info.disable_castle_kingside(&Player::White);
            game.info.disable_castle_kingside(&Player::Black);

            // Do setup moves
            for mv in &test_board.initial_moves {
                assert!(
                    do_move(&mut game, &mv).is_some(),
                    "move {} failed:\n{}",
                    mv,
                    game.board
                );
            }

            // Do move
            let mut rev_game = <ReversableGame as PlayableGame>::from_game(&mut game);

            assert!(
                rev_game.do_move(&test_board.mv),
                "invalid move {}:\n{}",
                test_board.mv,
                rev_game.as_ref().board
            );
        }
    }
}
