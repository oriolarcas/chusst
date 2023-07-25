// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod board;
mod moves;

use board::{Board, Piece, PieceType, Player, Position};

static BOARD: Board = Board {
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
};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn get_board() -> Board {
    BOARD
}

#[tauri::command]
fn get_possible_moves(row: usize, col: usize) -> Vec<Position> {
    let position = Position {row, col};
    println!("[Rust] Getting possible moves of {}", position);

    let possible_moves = moves::get_possible_moves(&BOARD, position);
    println!("Found {} moves", possible_moves.len());
    possible_moves
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_board, get_possible_moves])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
