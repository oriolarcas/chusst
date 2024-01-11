// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chusst::board::{Piece, Position};
use chusst::eval;
use chusst::eval::MateType;
use chusst::game::{Game, Move, MoveAction, MoveActionType, PromotionPieces};

use serde::Serialize;
use tauri::{LogicalSize, Manager, Size};

use std::sync::Mutex;

#[derive(Clone, Serialize)]
struct MoveDescription {
    mv: String,
    captures: Vec<Piece>,
    mate: Option<eval::MateType>,
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
fn get_possible_moves(rank: usize, file: usize) -> Vec<Position> {
    let position = Position { rank, file };
    let game = &mut GAME.lock().unwrap().game;
    let possible_moves =
        eval::get_possible_targets(&game.board, &game.last_move, &game.info, position);
    possible_moves
}

#[tauri::command]
fn get_possible_captures() -> eval::BoardCaptures {
    let game = &mut GAME.lock().unwrap().game;
    eval::get_possible_captures(&game)
}

#[tauri::command(rename_all = "snake_case")]
fn do_move(
    source_rank: usize,
    source_file: usize,
    target_rank: usize,
    target_file: usize,
    promotion: Option<String>,
) -> bool {
    let promotion_piece = promotion
        .map(|piece_str| PromotionPieces::try_from_str(piece_str))
        .flatten();
    let mv = MoveAction {
        mv: Move {
            source: Position {
                rank: source_rank,
                file: source_file,
            },
            target: Position {
                rank: target_rank,
                file: target_file,
            },
        },
        move_type: match promotion_piece {
            Some(piece) => MoveActionType::Promotion(piece),
            None => MoveActionType::Normal,
        },
    };
    let game_data = &mut GAME.lock().unwrap();
    let game = &mut game_data.game;

    let white_move = match eval::move_name(&game, &mv) {
        Some(name) => name,
        None => {
            println!("Invalid move: {}", mv.mv);
            return false;
        }
    };

    let white_captures = match eval::do_move(game, &mv) {
        Some(captures) => captures,
        None => {
            println!("Invalid move: {}", white_move);
            return false;
        }
    };

    let (black_move_opt, black_captures, mate) = match eval::get_best_move(game, 4) {
        eval::GameMove::Normal(mv) => {
            let description = eval::move_name(&game, &mv);

            let black_captures = eval::do_move(game, &mv);
            assert!(black_captures.is_some());

            let black_mate = eval::is_mate(&game.board, &game.player, &game.last_move, &game.info);

            (description, black_captures.unwrap(), black_mate)
        }
        eval::GameMove::Mate(mate) => match mate {
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
