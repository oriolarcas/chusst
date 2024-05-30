use divan::Bencher;

use chusst_gen::eval::{Game, GameHistory, SilentSearchFeedback};
use chusst_gen::game::BitboardGame;

#[divan::bench]
fn search(bench: Bencher) {
    bench.bench_local(|| {
        let game = BitboardGame::new();
        let history = GameHistory::new();

        let best_branch = game
            .get_best_move_recursive(4, &history, &mut (), &mut SilentSearchFeedback::default())
            .unwrap();

        best_branch.searched
    });
}

#[divan::bench_group]
mod perft {
    struct PerftDepth {
        pub depth: u8,
        pub moves: u64,
    }

    const PERFT_DEPTHS: [PerftDepth; 6] = [
        PerftDepth {
            depth: 1,
            moves: 20,
        },
        PerftDepth {
            depth: 2,
            moves: 400,
        },
        PerftDepth {
            depth: 3,
            moves: 8902,
        },
        PerftDepth {
            depth: 4,
            moves: 197281,
        },
        PerftDepth {
            depth: 5,
            moves: 4865609,
        },
        PerftDepth {
            depth: 6,
            moves: 119060324,
        },
    ];

    const PERFT_DEPTH: &PerftDepth = &PERFT_DEPTHS[3];

    #[divan::bench(items_count = PERFT_DEPTH.moves)]
    fn chusst() {
        use chusst_gen::eval::Game;
        use chusst_gen::game::BitboardGame;

        fn possible_moves_recursive(game: BitboardGame, depth: u8) {
            if depth == 0 {
                return;
            }

            for mv in game.get_all_possible_moves() {
                if depth == 1 {
                    continue;
                }
                let mut game = game.clone();
                game.do_move(&mv);
                possible_moves_recursive(game, depth - 1);
            }
        }

        possible_moves_recursive(BitboardGame::new(), PERFT_DEPTH.depth);
    }

    #[divan::bench(items_count = PERFT_DEPTH.moves)]
    fn chess() {
        use chess::{Board, MoveGen};

        fn possible_moves_recursive(board: Board, depth: u8) {
            if depth == 0 {
                return;
            }

            for mv in MoveGen::new_legal(&board) {
                if depth == 1 {
                    continue;
                }
                let board = board.make_move_new(mv);
                possible_moves_recursive(board, depth - 1);
            }
        }

        possible_moves_recursive(Board::default(), PERFT_DEPTH.depth);
    }

    #[divan::bench(items_count = PERFT_DEPTH.moves)]
    pub fn shackmaty() {
        use shakmaty::{Chess, Position};

        fn possible_moves_recursive(game: Chess, depth: u8) {
            if depth == 0 {
                return;
            }

            for mv in game.legal_moves() {
                if depth == 1 {
                    continue;
                }
                let mut game = game.clone();
                game.play_unchecked(&mv);
                possible_moves_recursive(game, depth - 1);
            }
        }

        possible_moves_recursive(Chess::default(), PERFT_DEPTH.depth);
    }
}

fn game_benchmark() -> u64 {
    // use std::io::Write;

    let mut game = BitboardGame::new();
    let mut history = GameHistory::new();

    let get_best_move_helper = |game: &mut BitboardGame, history: &mut GameHistory| {
        let best_branch = game
            .get_best_move_recursive(3, history, &mut (), &mut SilentSearchFeedback::default())
            .unwrap();

        (best_branch.searched, best_branch.moves.first().unwrap().mv)
    };

    // let format_mps = |mps: f64| {
    //     if mps >= 1000000. {
    //         format!("{:.3} M moves/s", mps / 1000000.)
    //     } else if mps >= 1000. {
    //         format!("{:.3} K moves/s", mps / 1000.)
    //     } else {
    //         format!("{:.0} moves/s", mps)
    //     }
    // };

    let mut total_searched: u64 = 0;

    for _ in 1..6 {
        // print!("{}. ", turn);
        // std::io::stdout().flush().unwrap();

        let (white_searched, white_move) = get_best_move_helper(&mut game, &mut history);
        // let white_move_name = move_name(&game.board, &game.last_move, &game.player, &white_move);
        game.do_move(&white_move);

        // print!("{} ", white_move_name);
        // std::io::stdout().flush().unwrap();

        let (black_searched, black_move) = get_best_move_helper(&mut game, &mut history);
        // let black_move_name = move_name(&game.board, &game.last_move, &game.player, &black_move);
        game.do_move(&black_move);

        // println!("{}", black_move_name);
        // std::io::stdout().flush().unwrap();

        total_searched += u64::from(white_searched) + u64::from(black_searched);
    }

    total_searched

    // println!("Searched: {}", total_searched);
    // println!(
    //     "Performance: {}",
    //     format_mps(f64::from(total_searched) / total_duration)
    // );
}

#[divan::bench]
fn game(bench: Bencher) {
    bench.bench_local(game_benchmark);
}

fn main() {
    divan::main();
}
