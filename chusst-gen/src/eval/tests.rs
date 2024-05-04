use super::play::PlayableGame;
use crate::board::{Board, ModifiableBoard, Piece, PieceType, Player, Position};
use crate::eval::check::SafetyChecks;
use crate::eval::Game;
use crate::game::{
    CastlingRights, GameState, ModifiableGame, Move, MoveAction, MoveActionType, PromotionPieces,
    SimpleGame,
};
use crate::{mva, p, pos};

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
    initial_moves: Vec<MoveAction>,
    mv: MoveAction,
    checks: Vec<PiecePosition>,
}

type TestGame = SimpleGame;

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

fn game_from_fen(fen: &str) -> TestGame {
    TestGame::try_from_fen(
        fen.split_ascii_whitespace()
            .collect::<Vec<&str>>()
            .as_slice(),
    )
    .unwrap_or_else(|| panic!("Failed to parse FEN string {}", fen))
}

#[test]
fn move_reversable() {
    let test_boards = [
        // Advance pawn
        TestBoard {
            board: None,
            initial_moves: vec![],
            mv: mva!(e2 => e3),
            checks: vec![pp!(pw @ e3), pp!(e2)],
        },
        // Pass pawn
        TestBoard {
            board: None,
            initial_moves: vec![],
            mv: mva!(e2 => e4),
            checks: vec![pp!(pw @ e4), pp!(e2)],
        },
        // Pawn capturing
        TestBoard {
            board: None,
            initial_moves: vec![mva!(e2 => e4), mva!(d7 => d5)],
            mv: mva!(e4 => d5),
            checks: vec![pp!(pw @ d5), pp!(e4)],
        },
        // Pawn capturing en passant
        TestBoard {
            board: None,
            initial_moves: vec![
                mva!(e2 => e4),
                mva!(a7 => a6),
                mva!(e4 => e5),
                mva!(d7 => d5),
            ],
            mv: mva!(e5 => d6),
            checks: vec![pp!(pw @ d6), pp!(e5), pp!(d5)],
        },
        // Pawn promotion to knight
        TestBoard {
            board: None,
            initial_moves: vec![
                mva!(h2 => h4),
                mva!(g7 => g6),
                mva!(h4 => h5),
                mva!(a7 => a6),
                mva!(h5 => g6),
                mva!(a6 => a5),
                mva!(g6 => g7),
                mva!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Knight),
            checks: vec![pp!(nw @ h8)],
        },
        // Pawn promotion to bishop
        TestBoard {
            board: None,
            initial_moves: vec![
                mva!(h2 => h4),
                mva!(g7 => g6),
                mva!(h4 => h5),
                mva!(a7 => a6),
                mva!(h5 => g6),
                mva!(a6 => a5),
                mva!(g6 => g7),
                mva!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Bishop),
            checks: vec![pp!(bw @ h8)],
        },
        // Pawn promotion to rook
        TestBoard {
            board: None,
            initial_moves: vec![
                mva!(h2 => h4),
                mva!(g7 => g6),
                mva!(h4 => h5),
                mva!(a7 => a6),
                mva!(h5 => g6),
                mva!(a6 => a5),
                mva!(g6 => g7),
                mva!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Rook),
            checks: vec![pp!(rw @ h8)],
        },
        // Pawn promotion to queen
        TestBoard {
            board: None,
            initial_moves: vec![
                mva!(h2 => h4),
                mva!(g7 => g6),
                mva!(h4 => h5),
                mva!(a7 => a6),
                mva!(h5 => g6),
                mva!(a6 => a5),
                mva!(g6 => g7),
                mva!(a5 => a4),
            ],
            mv: mva!(g7 => h8, PromotionPieces::Queen),
            checks: vec![pp!(qw @ h8)],
        },
        // Kingside castling
        TestBoard {
            board: None,
            initial_moves: vec![
                mva!(e2 => e3),
                mva!(a7 => a6),
                mva!(f1 => e2),
                mva!(b7 => b6),
                mva!(g1 => h3),
                mva!(c7 => c6),
            ],
            mv: mva!(e1 => g1),
            checks: vec![pp!(kw @ g1), pp!(rw @ f1)],
        },
        // Queenside castling
        TestBoard {
            board: None,
            initial_moves: vec![
                mva!(d2 => d4),
                mva!(a7 => a6),
                mva!(d1 => d3),
                mva!(b7 => b6),
                mva!(c1 => d2),
                mva!(c7 => c6),
                mva!(b1 => c3),
                mva!(d7 => d6),
            ],
            mv: mva!(e1 => c1),
            checks: vec![pp!(kw @ c1), pp!(rw @ d1)],
        },
    ];

    for test_board in &test_boards {
        // Prepare board
        let mut game: TestGame = custom_game(&test_board.board, Player::White);

        // Do setup moves
        for mv in &test_board.initial_moves {
            assert!(
                game.do_move(mv).is_some(),
                "move {} failed:\n{}",
                mv.mv,
                game.board()
            );
        }

        // Do move
        assert!(
            game.do_move_with_checks(&test_board.mv),
            "failed to make legal move {} in:\n{}",
            test_board.mv.mv,
            game.board()
        );

        for check in &test_board.checks {
            assert_eq!(
                game.as_ref().at(&check.position),
                check.piece,
                "expected {} in {}, found {}:\n{}",
                check
                    .piece
                    .map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                check.position,
                game.as_ref()
                    .at(&check.position)
                    .map_or("nothing".to_string(), |piece| format!("{}", piece.piece)),
                game.board(),
            );
        }
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
            initial_moves: vec![],
            mv: mva!(a5 => b3),
            checks: vec![],
        },
    ];

    for test_board in test_boards {
        // Prepare board
        let mut game: TestGame = custom_game(&test_board.board, Player::Black);

        game.disable_castle_kingside(Player::White);
        game.disable_castle_kingside(Player::Black);
        game.disable_castle_queenside(Player::White);
        game.disable_castle_queenside(Player::Black);

        // Do setup moves
        for mv in &test_board.initial_moves {
            assert!(
                game.do_move(mv).is_some(),
                "move {} failed:\n{}",
                mv.mv,
                game.board()
            );
        }

        let name = game.move_name(&test_board.mv).unwrap();
        assert!(
            name.ends_with('#'),
            "notation `{}` for move {} doesn't show checkmate sign # in:\n{}",
            name,
            test_board.mv.mv,
            game.board()
        );

        // Do move

        assert!(
            game.do_move_with_checks(&test_board.mv),
            "invalid move {}:\n{}",
            test_board.mv.mv,
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
    }
}

#[test]
fn fen_parsing() {
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
}

#[test]
fn perft() {
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

    let game = TestGame::new();

    assert_eq!(mv_rec(&game, 1), 20);
    assert_eq!(mv_rec(&game, 2), 400);
    assert_eq!(mv_rec(&game, 3), 8902);
    assert_eq!(mv_rec(&game, 4), 197281);

    // Perft position 3 from https://www.chessprogramming.org/Perft_Results
    let game = game_from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");

    assert_eq!(mv_rec(&game, 1), 14, "Perft 3 depth 1 expected 14");
    assert_eq!(mv_rec(&game, 2), 191, "Perft 3 depth 2 expected 191");
    assert_eq!(mv_rec(&game, 3), 2812, "Perft 3 depth 3 expected 2812");
    assert_eq!(mv_rec(&game, 4), 43238, "Perft 3 depth 4 expected 43238");

    // Perft position 4 from https://www.chessprogramming.org/Perft_Results
    let game = game_from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");

    assert_eq!(mv_rec(&game, 1), 6, "Perft 4 depth 1 expected 6");
    assert_eq!(mv_rec(&game, 2), 264, "Perft 4 depth 2 expected 264");
    assert_eq!(mv_rec(&game, 3), 9467, "Perft 4 depth 3 expected 9467");
    assert_eq!(mv_rec(&game, 4), 422333, "Perft 4 depth 4 expected 422333");

    // Perft position 5 from https://www.chessprogramming.org/Perft_Results
    let game = game_from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");

    assert_eq!(mv_rec(&game, 1), 44, "Perft 5 depth 1 expected 44");
    assert_eq!(mv_rec(&game, 2), 1486, "Perft 5 depth 2 expected 1486");
    assert_eq!(mv_rec(&game, 3), 62379, "Perft 5 depth 3 expected 62379");
    assert_eq!(
        mv_rec(&game, 4),
        2103487,
        "Perft 5 depth 4 expected 2103487"
    );

    // Perft position 6 from https://www.chessprogramming.org/Perft_Results
    let game =
        game_from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10");

    assert_eq!(mv_rec(&game, 1), 46, "Perft 6 depth 1 expected 46");
    assert_eq!(mv_rec(&game, 2), 2079, "Perft 6 depth 2 expected 2079");
    assert_eq!(mv_rec(&game, 3), 89890, "Perft 6 depth 3 expected 89890");
    assert_eq!(
        mv_rec(&game, 4),
        3894594,
        "Perft 6 depth 4 expected 3894594"
    );
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
        mv: mva!(e1 => c1),
        checks: vec![],
    }];

    for test_board in test_boards {
        // Prepare board
        let mut game: TestGame = custom_game(&test_board.board, Player::White);

        game.disable_castle_kingside(Player::White);
        game.disable_castle_kingside(Player::Black);

        // Do setup moves
        for mv in &test_board.initial_moves {
            assert!(
                game.do_move(mv).is_some(),
                "move {} failed:\n{}",
                mv.mv,
                game.board()
            );
        }

        // Do move
        assert!(
            game.do_move_with_checks(&test_board.mv),
            "invalid move {}:\n{}",
            test_board.mv.mv,
            game.as_ref().board()
        );
    }
}
