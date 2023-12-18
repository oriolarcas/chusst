// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chusst::board::{Game, Move, Piece, Position};
use chusst::moves;
use chusst::moves::MateType;

use serde::Serialize;
use tauri::{LogicalSize, Manager, Size};

use std::sync::Mutex;

#[derive(Clone, Serialize)]
struct MoveDescription {
    mv: String,
    captures: Vec<Piece>,
    mate: Option<moves::MateType>,
}

#[derive(Clone, Serialize)]
struct TurnDescription {
    white: MoveDescription,
    black: Option<MoveDescription>,
}

struct GameData {
    game: Game,
    history: Vec<TurnDescription>,
}

static GAME: Mutex<GameData> = Mutex::new(GameData {
    game: Game::new(),
    history: vec![],
});

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn get_game() -> Game {
    GAME.lock().unwrap().game
}

#[tauri::command]
fn get_history() -> Vec<TurnDescription> {
    GAME.lock().unwrap().history.clone()
}

#[tauri::command]
fn get_possible_moves(row: usize, col: usize) -> Vec<Position> {
    let position = Position { row, col };
    let game = &mut GAME.lock().unwrap().game;
    let possible_moves =
        moves::get_possible_moves(&game.board, &game.last_move, &game.info, position);
    possible_moves
}

#[tauri::command]
fn get_possible_captures() -> moves::BoardCaptures {
    let game = &mut GAME.lock().unwrap().game;
    moves::get_possible_captures(&game.board, &game.last_move, &game.info)
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
    let game_data = &mut GAME.lock().unwrap();
    let game = &mut game_data.game;

    let white_move =
        match moves::move_name(&game.board, &game.last_move, &game.info, &game.player, &mv) {
            Some(name) => name,
            None => {
                println!("Invalid move: {}", mv);
                return false;
            }
        };

    let white_captures = match moves::do_move(game, &mv) {
        Some(captures) => captures,
        None => {
            println!("Invalid move: {}", white_move);
            return false;
        }
    };

    let (black_move_opt, black_captures, mate) = match moves::get_best_move(game, 4) {
        moves::GameMove::Normal(mv) => {
            let description =
                moves::move_name(&game.board, &game.last_move, &game.info, &game.player, &mv);

            let black_captures = moves::do_move(game, &mv);
            assert!(black_captures.is_some());

            let black_mate = moves::is_mate(&game.board, &game.player, &game.last_move, &game.info);

            (description, black_captures.unwrap(), black_mate)
        }
        moves::GameMove::Mate(mate) => match mate {
            MateType::Stalemate => (None, vec![], Some(MateType::Stalemate)),
            MateType::Checkmate => (None, vec![], Some(MateType::Checkmate)),
        },
    };

    let history = &mut game_data.history;
    let turn = history.len() + 1;

    match black_move_opt {
        Some(black_move) => {
            println!("{}. {} {}", turn, white_move, black_move);

            history.push(TurnDescription {
                white: MoveDescription {
                    mv: white_move,
                    captures: white_captures,
                    mate: None,
                },
                black: Some(MoveDescription {
                    mv: black_move,
                    captures: black_captures,
                    mate,
                }),
            });
        }
        None => {
            println!("{}. {}", turn, white_move);

            history.push(TurnDescription {
                white: MoveDescription {
                    mv: white_move,
                    captures: white_captures,
                    mate,
                },
                black: None,
            });
        }
    }

    true
}

#[tauri::command]
fn restart() {
    let data = &mut GAME.lock().unwrap();

    data.game = Game::new();

    data.history.clear();

    println!("New game");
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
            get_history,
            get_possible_moves,
            get_possible_captures,
            do_move,
            restart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
