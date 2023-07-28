// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod board;
mod moves;

use board::{Board, Game, Piece, PieceType, Player, Position};

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
    turn: Player::White,
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
    println!(
        "Possible moves of {}: {} moves",
        position,
        possible_moves.len()
    );
    possible_moves
}

#[tauri::command(rename_all = "snake_case")]
fn do_move(source_row: usize, source_col: usize, target_row: usize, target_col: usize) -> bool {
    let source = Position {
        row: source_row,
        col: source_col,
    };
    let target = Position {
        row: target_row,
        col: target_col,
    };
    println!("Move {} -> {}", source, target);
    moves::do_move(&mut GAME.lock().unwrap(), source, target)
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
