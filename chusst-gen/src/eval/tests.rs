use super::play::PlayableGame;
use crate::board::{Board, ModifiableBoard, Piece, PieceType, Player, Position};
use crate::eval::check::SafetyChecks;
use crate::eval::Game;
use crate::game::{
    CastlingRights, GameState, ModifiableGame, Move, MoveAction, MoveActionType, PromotionPieces,
    SimpleGame,
};
use crate::{mva, p, pos};

enum Check {
    PiecePosition {
        piece: Option<Piece>,
        position: Position,
    },
    Action(Box<dyn Fn(&mut TestGame)>),
}

impl Check {
    pub fn from_action(action: impl Fn(&mut TestGame) + 'static) -> Check {
        Check::Action(Box::new(action))
    }
}

macro_rules! pp {
    ($piece:ident @ $pos:ident) => {
        Check::PiecePosition {
            piece: p!($piece),
            position: pos!($pos),
        }
    };
    ($pos:ident) => {
        Check::PiecePosition {
            piece: None,
            position: pos!($pos),
        }
    };
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum TestMove {
    AsMove(MoveAction),
    AsString(&'static str),
}

macro_rules! tm {
    ($src:ident => $tgt:ident) => {
        TestMove::AsMove(mva!($src => $tgt))
    };
    ($src:ident => $tgt:ident, $promote:ident) => {
        TestMove::AsMove(mva!($src => $tgt, $promote))
    };
    ($str:expr) => {
        TestMove::AsString($str)
    };
}

fn from_test_move(game: &TestGame, mv: &TestMove) -> MoveAction {
    match mv {
        TestMove::AsMove(mv) => *mv,
        TestMove::AsString(mv_str) => {
            let all_moves = game.get_all_possible_moves();
            let Some(mv) = all_moves
                .iter()
                .find(|mv| game.move_name(mv).unwrap().as_str() == *mv_str)
            else {
                panic!(
                    "move {} not found:\n{}\npossible moves: {}",
                    mv_str,
                    game.board(),
                    all_moves
                        .iter()
                        .map(|mv| game.move_name(mv).unwrap())
                        .collect::<Vec<String>>()
                        .join(", ")
                );
            };
            mv.to_owned()
        }
    }
}

struct TestBoard<'a> {
    board: Option<&'a str>,
    player: Player,
    initial_moves: Vec<TestMove>,
    mv: MoveAction,
    checks: Vec<Check>,
}

struct GameTestCase {
    initial_moves: Vec<TestMove>,
    mv: MoveAction,
    checks: Vec<Check>,
    game: TestGame,
}

impl GameTestCase {
    pub fn new(params: TestBoard) -> GameTestCase {
        let mut game: TestGame = Self::custom_game(&params.board, params.player);
        game.update_player(params.player);

        GameTestCase {
            initial_moves: params.initial_moves,
            mv: params.mv,
            checks: params.checks,
            game,
        }
    }
    pub fn do_initial_moves(&mut self) {
        for tm in &self.initial_moves {
            let mv = &from_test_move(&self.game, tm);
            assert!(
                self.game.do_move(mv).is_some(),
                "move {} failed:\n{}",
                mv.mv,
                self.game.board()
            );
        }
    }

    pub fn make_checks(&mut self) {
        for check in &self.checks {
            match check {
                Check::PiecePosition { piece, position } => {
                    assert_eq!(
                        &self.game.at(position),
                        piece,
                        "expected {} in {}, found {}:\n{}",
                        piece.map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                        position,
                        self.game
                            .as_ref()
                            .at(position)
                            .map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                        self.game.board(),
                    );
                }
                Check::Action(action) => {
                    action(&mut self.game);
                }
            }
        }
    }

    fn custom_game<B: Board>(board_opt: &Option<&str>, player: Player) -> GameState<B> {
        let mut game = match board_opt {
            Some(board_str) => {
                let mut board = B::default();

                let mut rank = 8usize;
                for line in board_str.lines() {
                    match line.find('[') {
                        Some(position) => {
                            rank -= 1;

                            for (file, piece_char) in line
                                .chars()
                                .skip(position)
                                .filter(|c| *c != '[' && *c != ']')
                                .enumerate()
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
                            }
                        }
                        None => continue,
                    }
                }
                GameState::from(board)
            }
            None => GameState::from(B::NEW_BOARD),
        };

        game.update_player(player);

        game
    }
}

type TestGame = SimpleGame;

struct MoveChain<'a> {
    game: &'a mut TestGame,
}

impl<'a> MoveChain<'a> {
    pub fn new(game: &'a mut TestGame) -> MoveChain<'a> {
        MoveChain { game }
    }

    pub fn do_move(&mut self, mv: TestMove) -> &mut Self {
        let mv = &from_test_move(self.game, &mv);
        assert!(
            self.game.do_move(mv).is_some(),
            "move {} failed:\n{}",
            mv.mv,
            self.game.board()
        );
        self
    }
}

fn game_from_fen(fen: &str) -> TestGame {
    TestGame::try_from_fen(
        fen.split_ascii_whitespace()
            .collect::<Vec<&str>>()
            .as_slice(),
    )
    .unwrap_or_else(|| panic!("Failed to parse FEN string {}", fen))
}

impl From<shakmaty::Square> for Position {
    fn from(pos: shakmaty::Square) -> Self {
        pos!(pos.rank() as usize, pos.file() as usize)
    }
}

impl From<shakmaty::Move> for MoveAction {
    fn from(mv: shakmaty::Move) -> Self {
        match mv {
            shakmaty::Move::Normal {
                from,
                to,
                promotion,
                ..
            } => {
                let from = from.into();
                let to = to.into();
                match promotion {
                    Some(shakmaty::Role::Knight) => mva!(from, to, PromotionPieces::Knight),
                    Some(shakmaty::Role::Bishop) => mva!(from, to, PromotionPieces::Bishop),
                    Some(shakmaty::Role::Rook) => mva!(from, to, PromotionPieces::Rook),
                    Some(shakmaty::Role::Queen) => mva!(from, to, PromotionPieces::Queen),
                    _ => mva!(from, to),
                }
            }
            shakmaty::Move::Castle { king, rook } => {
                if rook.file() as usize == 0 {
                    mva!(king.into(), pos!(king.rank().into(), 2))
                } else {
                    mva!(king.into(), pos!(king.rank().into(), 6))
                }
            }
            shakmaty::Move::EnPassant { from, to } => mva!(from.into(), to.into()),
            _ => panic!("Shakmaty move not supported"),
        }
    }
}

fn perft_compare_against_shakmaty(fen: &str, depth: u8) {
    let chusst_game = game_from_fen(fen);
    let shakmaty_game = fen
        .parse::<shakmaty::fen::Fen>()
        .expect("Failed to parse FEN string")
        .into_position::<shakmaty::Chess>(shakmaty::CastlingMode::Standard)
        .expect("Failed to convert FEN to position");

    fn perft_compare(
        chusst_game: TestGame,
        shakmaty_game: shakmaty::Chess,
        depth: u8,
        moves: &[&TestGame],
    ) {
        use shakmaty::Position;
        use std::collections::HashMap;

        if depth == 0 {
            return;
        }

        let chusst_moves = chusst_game.get_all_possible_moves();
        let shakmaty_moves = shakmaty_game.legal_moves();
        let shakmaty_moves_map: HashMap<shakmaty::Move, MoveAction> = HashMap::from_iter(
            shakmaty_moves
                .iter()
                .map(|mv| (mv.clone(), MoveAction::from(mv.clone()))),
        );

        // Compare the list of moves and panic if they don't match, displaying the moves that are different
        let chusst_moves_not_in_shakmaty = chusst_moves
            .iter()
            .filter(|mv| !shakmaty_moves_map.values().collect::<Vec<_>>().contains(mv))
            .cloned()
            .collect::<Vec<_>>();
        let shakmaty_moves_not_in_chusst = shakmaty_moves_map
            .values()
            .filter(|mv| !chusst_moves.contains(mv))
            .cloned()
            .collect::<Vec<_>>();

        if !chusst_moves_not_in_shakmaty.is_empty() || !shakmaty_moves_not_in_chusst.is_empty() {
            fn format_mv_list<'a, I>(moves: I) -> String
            where
                I: Iterator<Item = &'a MoveAction>,
            {
                moves
                    .map(|mv| format!("{}", mv.mv))
                    .collect::<Vec<_>>()
                    .join(", ")
            }

            let shakmaty_moves_not_in_chusst_names = shakmaty_moves_not_in_chusst
                .iter()
                .map(|mv| {
                    format!(
                        "{}",
                        shakmaty_moves_map.iter().find(|(_, v)| *v == mv).unwrap().0
                    )
                })
                .collect::<Vec<_>>();

            for game in moves {
                println!("After:\n{}", game.board());
            }

            panic!(
                "Player {} in board:\n{}\nChusst moves: [{}]\nShakmaty moves: [{}]\nMoves not in Shakmaty: [{}]\nMoves not in Chusst: [{}]\n                     [{}]",
                chusst_game.player(),
                chusst_game.board(),
                format_mv_list(chusst_moves.iter()),
                format_mv_list(shakmaty_moves_map.values()),
                format_mv_list(chusst_moves_not_in_shakmaty.iter()),
                format_mv_list(shakmaty_moves_not_in_chusst.iter()),
                shakmaty_moves_not_in_chusst_names.join(", "),
            );
        }

        for chusst_mv in chusst_moves.iter() {
            let mut new_chusst_game = chusst_game.clone();
            let mut shakmaty_game = shakmaty_game.clone();
            let shakmaty_mv = shakmaty_moves_map
                .iter()
                .find(|(_, mv)| *mv == chusst_mv)
                .unwrap()
                .0;

            new_chusst_game.do_move(chusst_mv);
            shakmaty_game.play_unchecked(shakmaty_mv);

            let moves = Vec::from_iter(moves.iter().chain(&[&chusst_game]).copied());

            perft_compare(new_chusst_game, shakmaty_game, depth - 1, &moves);
        }
    }

    perft_compare(chusst_game.clone(), shakmaty_game, depth, &[]);
}

#[test]
fn move_reversable() {
    let test_boards = [
        // Advance pawn
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(e2 => e3),
            checks: vec![pp!(pw @ e3), pp!(e2)],
        },
        // Pass pawn
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(e2 => e4),
            checks: vec![pp!(pw @ e4), pp!(e2)],
        },
        // Pawn capturing
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![tm!(e2 => e4), tm!(d7 => d5)],
            mv: mva!(e4 => d5),
            checks: vec![pp!(pw @ d5), pp!(e4)],
        },
        // Pawn capturing en passant
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![tm!(e2 => e4), tm!(a7 => a6), tm!(e4 => e5), tm!(d7 => d5)],
            mv: mva!(e5 => d6),
            checks: vec![pp!(pw @ d6), pp!(e5), pp!(d5)],
        },
        // Pawn promotion to knight
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![
                tm!(h2 => h4),
                tm!(g7 => g6),
                tm!(h4 => h5),
                tm!(a7 => a6),
                tm!(h5 => g6),
                tm!(a6 => a5),
                tm!(g6 => g7),
                tm!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Knight),
            checks: vec![pp!(nw @ h8)],
        },
        // Pawn promotion to bishop
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![
                tm!(h2 => h4),
                tm!(g7 => g6),
                tm!(h4 => h5),
                tm!(a7 => a6),
                tm!(h5 => g6),
                tm!(a6 => a5),
                tm!(g6 => g7),
                tm!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Bishop),
            checks: vec![pp!(bw @ h8)],
        },
        // Pawn promotion to rook
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![
                tm!(h2 => h4),
                tm!(g7 => g6),
                tm!(h4 => h5),
                tm!(a7 => a6),
                tm!(h5 => g6),
                tm!(a6 => a5),
                tm!(g6 => g7),
                tm!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Rook),
            checks: vec![pp!(rw @ h8)],
        },
        // Pawn promotion to queen
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![
                tm!(h2 => h4),
                tm!(g7 => g6),
                tm!(h4 => h5),
                tm!(a7 => a6),
                tm!(h5 => g6),
                tm!(a6 => a5),
                tm!(g6 => g7),
                tm!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Queen),
            checks: vec![pp!(qw @ h8)],
        },
        // Kingside castling
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![
                tm!(e2 => e3),
                tm!(a7 => a6),
                tm!(f1 => e2),
                tm!(b7 => b6),
                tm!(g1 => h3),
                tm!(c7 => c6),
            ],
            mv: mva!(e1 => g1),
            checks: vec![pp!(kw @ g1), pp!(rw @ f1)],
        },
        // Queenside castling
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![
                tm!(d2 => d4),
                tm!(a7 => a6),
                tm!(d1 => d3),
                tm!(b7 => b6),
                tm!(c1 => d2),
                tm!(c7 => c6),
                tm!(b1 => c3),
                tm!(d7 => d6),
            ],
            mv: mva!(e1 => c1),
            checks: vec![pp!(kw @ c1), pp!(rw @ d1)],
        },
    ];

    for test_board in test_boards {
        let mut test_case = GameTestCase::new(test_board);

        // Do setup moves
        test_case.do_initial_moves();
        let game = &mut test_case.game;

        // Do move
        assert!(
            game.do_move_with_checks(&test_case.mv),
            "failed to make legal move {} in:\n{}",
            test_case.mv.mv,
            game.board()
        );

        test_case.make_checks();
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
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(b3 => b2),
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
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(b3 => b2),
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
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(b8 => a8),
            checks: vec![],
        },
        TestBoard {
            board: Some(
                "  a  b  c  d  e  f  g  h \n\
                8 [ ][ ][ ][ ][ ][♝][♝][♚]\n\
                7 [ ][ ][ ][ ][ ][ ][ ][♝]\n\
                6 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                5 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                4 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                3 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                2 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                1 [♔][ ][ ][ ][ ][ ][ ][ ]",
            ),
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(f8 => g7),
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
                2 [ ][ ][ ][ ][ ][ ][ ][ ]\n\
                1 [♔][ ][ ][♞][ ][ ][ ][ ]",
            ),
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(a5 => b3),
            checks: vec![],
        },
    ];

    for test_board in test_boards {
        // Prepare board
        let mut test_case = GameTestCase::new(test_board);

        test_case.game.disable_castle_kingside(Player::White);
        test_case.game.disable_castle_kingside(Player::Black);
        test_case.game.disable_castle_queenside(Player::White);
        test_case.game.disable_castle_queenside(Player::Black);

        // Do setup moves
        test_case.do_initial_moves();

        let game = &mut test_case.game;

        let name = match game.move_name(&test_case.mv) {
            Ok(name) => name,
            Err(err) => panic!(
                "no move name for {}: {}\n{}",
                test_case.mv.mv,
                err,
                game.board()
            ),
        };
        assert!(
            name.ends_with('#'),
            "notation `{}` for move {} doesn't show checkmate sign # in:\n{}",
            name,
            test_case.mv.mv,
            game.board()
        );

        // Do move

        assert!(
            game.do_move_with_checks(&test_case.mv),
            "invalid move {}:\n{}",
            test_case.mv.mv,
            game.board()
        );

        let possible_moves = game.get_possible_moves(pos!(a1));
        let in_check = game.board().is_piece_unsafe(&pos!(a1));
        assert!(in_check, "king should be in check:\n{}", game.board());
        assert!(
            possible_moves.is_empty(),
            "unexpected possible move {} in check mate:\n{}",
            possible_moves.first().unwrap().mv,
            game.board()
        );

        test_case.make_checks();
    }
}

#[test]
fn zobrist() {
    // Deterministic hash
    assert_eq!(TestGame::new().hash(), TestGame::new().hash());

    let test_boards = [
        // Hash changes after move
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![],
            mv: mva!(e2 => e3),
            checks: vec![Check::from_action(|game| {
                assert_ne!(
                    game.hash(),
                    TestGame::new().hash(),
                    "hash should change after move:\n{}",
                    game.board()
                );
            })],
        },
        // Castling rights
        TestBoard {
            board: None,
            player: Player::White,
            initial_moves: vec![tm!("Nf3")],
            mv: mva!(a7 => a6),
            checks: vec![Check::from_action(|game| {
                let hash_before = game.hash();
                game.do_move(&mva!(h1 => g1)); // moving rook loses kingside castling rights
                game.do_move(&mva!(b7 => b6));
                game.do_move(&mva!(g1 => h1)); // return rook to original position

                assert_ne!(
                    game.hash(),
                    hash_before,
                    "hash should change after castling:\n{}",
                    game.board()
                );
            })],
        },
    ];

    for test_board in test_boards {
        let mut test_case = GameTestCase::new(test_board);

        // Do setup moves
        test_case.do_initial_moves();
        let game = &mut test_case.game;

        // Do move
        assert!(
            game.do_move_with_checks(&test_case.mv),
            "invalid move {}:\n{}",
            test_case.mv.mv,
            game.board()
        );

        test_case.make_checks();
    }

    // Order of moves doesn't matter
    let mut game1 = TestGame::new();
    let mut game2 = TestGame::new();
    MoveChain::new(&mut game1)
        .do_move(tm!("e3"))
        .do_move(tm!("e6"))
        .do_move(tm!("d3"));
    MoveChain::new(&mut game2)
        .do_move(tm!("d3"))
        .do_move(tm!("e6"))
        .do_move(tm!("e3"));
    assert_eq!(
        game1.hash(),
        game2.hash(),
        "hash should be the same after same moves in different order:\n{}\n{}",
        game1.board(),
        game2.board(),
    );

    // En passant
    let mut game1 = TestGame::new();
    let mut game2 = TestGame::new();
    MoveChain::new(&mut game1)
        .do_move(tm!("e4")) // en passant active at file e
        .do_move(tm!("e6")) // en passant deactivated
        .do_move(tm!("d3")); // no en passant file
    MoveChain::new(&mut game2)
        .do_move(tm!("d3")) // no en passant file
        .do_move(tm!("e6")) // no en passant file
        .do_move(tm!("e4")); // en passant active at file e
    assert_ne!(
        game1.hash(),
        game2.hash(),
        "hash should be different with different en passant files active:\n{}\n{}",
        game1.board(),
        game2.board(),
    );

    // Getting the hash after making the moves should be the same
    let mut game1 = TestGame::new();
    let mut game2 = TestGame::new();

    // game1 has a hash from the beginning
    game1.hash();

    MoveChain::new(&mut game1).do_move(tm!("e4"));
    MoveChain::new(&mut game2).do_move(tm!("e4"));
    let hash1 = game1.hash();
    let hash2 = game2.hash(); // game2 gets the hash from scratch after making the moves
    assert_eq!(
        hash1,
        hash2,
        "hash should be the same after if obtained after making the moves:\n{}\n{}",
        game1.board(),
        game2.board(),
    );
}

#[test]
fn fen() {
    // Parsing
    let start_pos_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let parsed_game = TestGame::try_from_fen(
        start_pos_fen
            .split_ascii_whitespace()
            .collect::<Vec<&str>>()
            .as_slice(),
    );
    assert!(parsed_game.is_some(), "Failed to parse FEN string");
    let game = parsed_game.unwrap();
    assert_eq!(game, TestGame::new(), "\n{}", game.board());

    // Generating
    assert_eq!(
        game.to_fen(),
        start_pos_fen,
        "FEN string should match the original:\n{}\n{}",
        start_pos_fen,
        game.to_fen()
    );
}

fn perft_impl(force_comparison: bool) {
    fn mv_rec(game: &TestGame, depth: u8) -> u64 {
        if depth == 0 {
            return 1;
        }

        let mut nodes = 0;
        for mv in game.get_all_possible_moves() {
            if depth == 1 {
                nodes += 1;
                continue;
            }
            let mut game_copy = game.clone();
            game_copy.do_move(&mv);
            nodes += mv_rec(&game_copy, depth - 1);
        }

        nodes
    }

    let assert_perft = |fen: &str, name: &str, depth: u8, expected: u64| {
        let game = game_from_fen(fen);

        if force_comparison {
            perft_compare_against_shakmaty(fen, depth);
            return;
        }

        let nodes = mv_rec(&game, depth);
        if nodes != expected {
            println!(
                "Perft {} depth {} expected {}, got {}",
                name, depth, expected, nodes
            );
            println!("Comparing against Shakmaty:");

            perft_compare_against_shakmaty(fen, depth);

            panic!("Perft failed"); // just in case the comparison didn't panic
        }
    };

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    assert_perft(fen, "position 1", 1, 20);
    assert_perft(fen, "position 1", 2, 400);
    assert_perft(fen, "position 1", 3, 8902);
    assert_perft(fen, "position 1", 4, 197281);

    // Perft position 3 from https://www.chessprogramming.org/Perft_Results
    let fen = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";

    assert_perft(fen, "position 3", 1, 14);
    assert_perft(fen, "position 3", 2, 191);
    assert_perft(fen, "position 3", 3, 2812);
    assert_perft(fen, "position 3", 4, 43238);

    // Perft position 4 from https://www.chessprogramming.org/Perft_Results
    let fen = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";

    assert_perft(fen, "position 4", 1, 6);
    assert_perft(fen, "position 4", 2, 264);
    assert_perft(fen, "position 4", 3, 9467);
    assert_perft(fen, "position 4", 4, 422333);

    // Perft position 5 from https://www.chessprogramming.org/Perft_Results
    let fen = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";

    assert_perft(fen, "position 5", 1, 44);
    assert_perft(fen, "position 5", 2, 1486);
    assert_perft(fen, "position 5", 3, 62379);
    assert_perft(fen, "position 5", 4, 2103487);

    // Perft position 6 from https://www.chessprogramming.org/Perft_Results
    let fen = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

    assert_perft(fen, "position 6", 1, 46);
    assert_perft(fen, "position 6", 2, 2079);
    assert_perft(fen, "position 6", 3, 89890);
    assert_perft(fen, "position 6", 4, 3894594);
}

#[test]
fn perft() {
    perft_impl(false);
}

#[test]
#[ignore]
fn perft_slow() {
    perft_impl(true);
}

// Template to quickly test a specific board/move
#[test]
#[ignore]
fn quick_test() {
    // White: ♙ ♘ ♗ ♖ ♕ ♔
    // Black: ♟ ♞ ♝ ♜ ♛ ♚
    #[rustfmt::skip]
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
        player: Player::White,
        initial_moves: vec![],
        mv: mva!(e1 => c1),
        checks: vec![],
    }];

    for test_board in test_boards {
        // Prepare board
        let mut test_case = GameTestCase::new(test_board);

        test_case.game.disable_castle_kingside(Player::White);
        test_case.game.disable_castle_kingside(Player::Black);

        // Do setup moves
        test_case.do_initial_moves();

        let game = &mut test_case.game;

        // Do move
        assert!(
            game.do_move_with_checks(&test_case.mv),
            "invalid move {}:\n{}",
            test_case.mv.mv,
            game.as_ref().board()
        );
    }
}
