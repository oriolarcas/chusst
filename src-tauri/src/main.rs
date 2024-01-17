// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chusst_gen::board::{Piece, Position};
use chusst_gen::eval::{self, Game};
use chusst_gen::game::{Move, MoveAction, MoveActionType, PromotionPieces};

#[cfg(feature = "bitboards")]
type GameModel = chusst_gen::game::BitboardGame;
#[cfg(feature = "compact-board")]
type GameModel = chusst_gen::game::CompactGame;
#[cfg(all(not(feature = "bitboards"), not(feature = "compact-board")))]
type GameModel = chusst_gen::game::SimpleGame;

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
    game: GameModel,
    history: Vec<TurnDescription>,
}

static GAME: Mutex<GameData> = Mutex::new(GameData {
    game: GameModel::new(),
    history: vec![],
});

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn get_game() -> GameModel {
    GAME.lock().unwrap().game.clone()
}

#[tauri::command]
fn get_history() -> Vec<TurnDescription> {
    GAME.lock().unwrap().history.clone()
}

#[tauri::command]
fn get_possible_moves(rank: usize, file: usize) -> Vec<Position> {
    let position = Position { rank, file };
    let game = &mut GAME.lock().unwrap().game;
    game.get_possible_targets(position)
}

#[tauri::command]
fn get_possible_captures() -> eval::BoardCaptures {
    let game = &mut GAME.lock().unwrap().game;
    game.get_possible_captures()
}

#[tauri::command(rename_all = "snake_case")]
fn do_move(
    source_rank: usize,
    source_file: usize,
    target_rank: usize,
    target_file: usize,
    promotion: Option<String>,
) -> bool {
    let promotion_piece = promotion.and_then(PromotionPieces::try_from_str);
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

    let white_move = match game.move_name(&mv) {
        Some(name) => name,
        None => {
            println!("Invalid move: {}", mv.mv);
            return false;
        }
    };

    let white_captures = match game.do_move(&mv) {
        Some(captures) => captures,
        None => {
            println!("Invalid move: {}", white_move);
            return false;
        }
    };

    let (black_move_opt, black_captures, mate) = match game.get_best_move(4) {
        eval::GameMove::Normal(mv) => {
            let description = game.move_name(&mv);

            let black_captures = game.do_move(&mv);
            assert!(black_captures.is_some());

            let black_mate = game.is_mate();

            (description, black_captures.unwrap(), black_mate)
        }
        eval::GameMove::Mate(mate) => match mate {
            eval::MateType::Stalemate => (None, vec![], Some(eval::MateType::Stalemate)),
            eval::MateType::Checkmate => (None, vec![], Some(eval::MateType::Checkmate)),
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

    data.game = GameModel::new();

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
