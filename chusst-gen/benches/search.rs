use chusst_gen::eval::{Game, SilentSearchFeedback};
use chusst_gen::game::BitboardGame;

#[macro_use]
extern crate bencher;

use bencher::Bencher;

fn search(bench: &mut Bencher) {
    let mut searched = 0u64;
    bench.iter(|| {
        let game = BitboardGame::new();

        let best_branch = game
            .get_best_move_recursive(4, &mut (), &mut SilentSearchFeedback::default())
            .unwrap();

        searched = u64::from(best_branch.searched);
    });

    bench.bytes = searched;
}

fn perft4(bench: &mut Bencher) {
    let mut searched = 0u64;

    fn possible_moves_recursive(game: &BitboardGame, depth: u8) -> u64 {
        if depth == 0 {
            return 1;
        }

        let mut count = 0;

        let moves = game.get_all_possible_moves();
        for mv in moves {
            let mut game = game.clone();
            game.do_move(&mv);
            count += possible_moves_recursive(&game, depth - 1);
        }

        count
    }

    bench.iter(|| {
        let game = BitboardGame::new();

        searched = possible_moves_recursive(&game, 4);
    });

    bench.bytes = searched;
}

fn game_benchmark() -> u64 {
    // use std::io::Write;

    let mut game = BitboardGame::new();
    let get_best_move_helper = |game: &mut BitboardGame| {
        let best_branch = game
            .get_best_move_recursive(3, &mut (), &mut SilentSearchFeedback::default())
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

        let (white_searched, white_move) = get_best_move_helper(&mut game);
        // let white_move_name = move_name(&game.board, &game.last_move, &game.player, &white_move);
        game.do_move(&white_move);

        // print!("{} ", white_move_name);
        // std::io::stdout().flush().unwrap();

        let (black_searched, black_move) = get_best_move_helper(&mut game);
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

fn game(bench: &mut Bencher) {
    let mut searched = 0u64;
    bench.iter(|| {
        searched = game_benchmark();
    });
    bench.bytes = searched;
}

benchmark_group!(benches, game, search, perft4);
benchmark_main!(benches);
