// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chusst_gen::board::{Piece, Position};
use chusst_gen::eval::{self, Game, GameHistory};
use chusst_gen::game::{Move, MoveAction, MoveActionType, PromotionPieces};

#[cfg(feature = "bitboards")]
type GameModel = chusst_gen::game::BitboardGame;
#[cfg(feature = "compact-board")]
type GameModel = chusst_gen::game::CompactGame;
#[cfg(all(not(feature = "bitboards"), not(feature = "compact-board")))]
type GameModel = chusst_gen::game::SimpleGame;

use serde::Serialize;

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
    /// The game state
    game: GameModel,
    /// The game history required by the engine
    move_history: GameHistory,
    /// The game history required by the UI
    history: Vec<TurnDescription>,
}

static GAME: Mutex<GameData> = Mutex::new(GameData {
    game: GameModel::new(),
    move_history: Vec::new(),
    history: Vec::new(),
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
    let promotion_piece = if let Some(promotion_value) = promotion {
        let promotion_piece = PromotionPieces::try_from_str(promotion_value.clone());
        if promotion_piece.is_none() {
            println!("Invalid promotion piece: {}", promotion_value);
            return false;
        }
        promotion_piece
    } else {
        None
    };
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

    let white_move = match game_data.game.move_name(&mv) {
        Ok(name) => name,
        Err(err) => {
            println!("Invalid move {}: {}", mv.mv, err);
            return false;
        }
    };

    let white_captures = match game_data.game.do_move(&mv) {
        Some(captures) => captures,
        None => {
            println!("Invalid move: {}", white_move);
            return false;
        }
    };

    game_data.move_history.push(mv);

    let (black_move_opt, black_captures, mate) =
        match game_data.game.get_best_move(&game_data.move_history, 4) {
            eval::GameMove::Normal(mv) => {
                let description = game_data.game.move_name(&mv).ok();

                let black_captures = game_data.game.do_move(&mv);
                assert!(black_captures.is_some());

                game_data.move_history.push(mv);

                let black_mate = game_data.game.is_mate();

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
