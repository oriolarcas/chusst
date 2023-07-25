// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;

#[derive(Copy, Clone, Serialize)]
enum PieceType {
    Pawn,   // p
    Knight, // n
    Bishop, // b
    Rook,   // r
    Queen,  // q
    King,   // k
}

#[derive(Copy, Clone, Serialize)]
enum Player {
    White,
    Black,
}

#[derive(Copy, Clone, Serialize)]
struct Piece {
    piece: PieceType,
    player: Player,
}

macro_rules! p {
    (pw) => {
        Some(Piece {
            piece: PieceType::Pawn,
            player: Player::White,
        })
    };
    (nw) => {
        Some(Piece {
            piece: PieceType::Knight,
            player: Player::White,
        })
    };
    (bw) => {
        Some(Piece {
            piece: PieceType::Bishop,
            player: Player::White,
        })
    };
    (rw) => {
        Some(Piece {
            piece: PieceType::Rook,
            player: Player::White,
        })
    };
    (qw) => {
        Some(Piece {
            piece: PieceType::Queen,
            player: Player::White,
        })
    };
    (kw) => {
        Some(Piece {
            piece: PieceType::King,
            player: Player::White,
        })
    };
    (pb) => {
        Some(Piece {
            piece: PieceType::Pawn,
            player: Player::Black,
        })
    };
    (nb) => {
        Some(Piece {
            piece: PieceType::Knight,
            player: Player::Black,
        })
    };
    (bb) => {
        Some(Piece {
            piece: PieceType::Bishop,
            player: Player::Black,
        })
    };
    (rb) => {
        Some(Piece {
            piece: PieceType::Rook,
            player: Player::Black,
        })
    };
    (qb) => {
        Some(Piece {
            piece: PieceType::Queen,
            player: Player::Black,
        })
    };
    (kb) => {
        Some(Piece {
            piece: PieceType::King,
            player: Player::Black,
        })
    };
    () => {
        Option::<Piece>::None
    };
}

#[derive(Copy, Clone, Serialize)]
struct Board {
    // rows[x][y], where x = 0..7 = rows 1..8, and y = 0..7 = columns a..h
    // for instance, e4 is Board.rows[2][4]
    rows: [[Option<Piece>; 8]; 8],
}

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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_board])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
