// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod board;
mod moves;

use board::{Board, Game, Move, Piece, PieceType, Player, Position};

use tauri::{LogicalSize, Manager, Size};

use std::sync::Mutex;

static GAME: Mutex<Game> = Mutex::new(Game {
    board: Board {
        // Initial board
        // Note that white pieces are at the top, because arrays are defined top-down, while chess rows go bottom-up
        rows: [
            [
                p!(rw),
                p!(nw),
                p!(bw),
                p!(qw),
                p!(kw),
                p!(bw),
                p!(nw),
                p!(rw),
            ],
            [
                p!(pw),
                p!(pw),
                p!(pw),
                p!(pw),
                p!(pw),
                p!(pw),
                p!(pw),
                p!(pw),
            ],
            [p!(); 8],
            [p!(); 8],
            [p!(); 8],
            [p!(); 8],
            [
                p!(pb),
                p!(pb),
                p!(pb),
                p!(pb),
                p!(pb),
                p!(pb),
                p!(pb),
                p!(pb),
            ],
            [
                p!(rb),
                p!(nb),
                p!(bb),
                p!(qb),
                p!(kb),
                p!(bb),
                p!(nb),
                p!(rb),
            ],
        ],
    },
    player: Player::White,
    turn: 1,
});

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn get_game() -> Game {
    *GAME.lock().unwrap()
}

#[tauri::command]
fn get_possible_moves(row: usize, col: usize) -> Vec<Position> {
    let position = Position { row, col };
    let possible_moves = moves::get_possible_moves(&GAME.lock().unwrap().board, position);
    // println!(
    //     "Possible moves of {}: {} moves",
    //     position,
    //     possible_moves.len()
    // );
    possible_moves
}

#[tauri::command(rename_all = "snake_case")]
fn do_move(source_row: usize, source_col: usize, target_row: usize, target_col: usize) -> bool {
    let mv = Move {
        source: Position {
            row: source_row,
            col: source_col,
        },
        target: Position {
            row: target_row,
            col: target_col,
        },
    };
    let game = &mut GAME.lock().unwrap();

    let white_move = moves::move_name(&game.board, &game.player, &mv);

    if !moves::do_move(game, mv) {
        println!("Invalid move: {}", white_move);
        return false;
    }

    match moves::get_best_move(&game.board, &game.player) {
        Some(mv) => {
            println!("{}. {} {}", game.turn, white_move, moves::move_name(&game.board, &game.player, &mv));
            assert!(moves::do_move(game, mv));
        }
        None => println!("{}: no move?!", game.turn),
    }

    game.turn += 1;

    true
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();
            main_window.set_size(Size::Logical(LogicalSize {
                width: 1000.0,
                height: 800.0,
            }))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_game,
            get_possible_moves,
            do_move
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
