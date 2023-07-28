const { invoke } = window.__TAURI__.tauri;

let game = {
  board: {rows: [
    new Array(8).fill(null),
    new Array(8).fill(null),
    new Array(8).fill(null),
    new Array(8).fill(null),
    new Array(8).fill(null),
    new Array(8).fill(null),
    new Array(8).fill(null),
    new Array(8).fill(null),
  ]},
  turn: null,
};

const WHITE = "white";
const BLACK = "black";

let selected = null;

function get_square(row_index, col_index) {
  const col_id_map = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
  const row_id = row_index + 1;
  const col_id = col_id_map[col_index];
  return document.querySelector(".row-" + row_id + " .col-" + col_id);
}

function get_piece(row_index, col_index) {
  const col_id_map = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
  const row_id = row_index + 1;
  const col_id = col_id_map[col_index];
  return document.querySelector(".row-" + row_id + " .col-" + col_id + " div");
}

function apply_to_squares(func) {
  for (let row_index = 0; row_index < 8; row_index++) {
    for (let col_index = 0; col_index < 8; col_index++) {
      func(get_square(row_index, col_index), row_index, col_index);
    }
  }
}

function apply_to_pieces(func) {
  for (let row_index = 0; row_index < 8; row_index++) {
    for (let col_index = 0; col_index < 8; col_index++) {
      func(get_piece(row_index, col_index), row_index, col_index);
    }
  }
}

function reset_highlighted(highlight_type) {
  apply_to_squares((square) => {
    if (highlight_type !== undefined) {
      square.classList.remove("highlight-" + highlight_type);
      return;
    }
    square.classList.remove("highlight-source");
    square.classList.remove("highlight-move");
    square.classList.remove("highlight-capture");
    // square.classList.remove("highlight-selected");
  });
}

function highlight_square(row_index, col_index, hightlight_type) {
  let square = get_square(row_index, col_index);
  square.classList.add("highlight-" + hightlight_type);
}

function is_square_empty(row, col) {
  return game.board.rows[row][col] === null;
}

function is_square_player(row, col, player) {
  const square = game.board.rows[row][col];
  if (square === null) {
    return false;
  }
  return square.player.toLowerCase() == player.toLowerCase();
}

function is_square_selected(row, col) {
  if (selected === null) {
    return false;
  }
  return selected.row == row && selected.col == col;
}

async function get_game() {
  console.log("Reloading board");
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  game = await invoke("get_game");

  // Object layout:
  // { rows: [
  //   0 [{piece: "Rook", player: "White"}, {piece: "Knight", player: "White"}, {piece: "Bishop", player: "White"}, {piece: "Queen", player: "White"}, {piece: "King", player: "White"}, {piece: "Bishop", player: "White"}, {piece: "Knight", player: "White"}, {piece: "Rook", player: "White"}]
  //   1 [{piece: "Pawn", player: "White"}, {piece: "Pawn", player: "White"}, {piece: "Pawn", player: "White"}, {piece: "Pawn", player: "White"}, {piece: "Pawn", player: "White"}, {piece: "Pawn", player: "White"}, {piece: "Pawn", player: "White"}, {piece: "Pawn", player: "White"}]
  //   2 [null, null, null, null, null, null, null, null]
  //   3 [null, null, null, null, null, null, null, null]
  //   4 [null, null, null, null, null, null, null, null]
  //   5 [null, null, null, null, null, null, null, null]
  //   6 [{piece: "Pawn", player: "Black"}, {piece: "Pawn", player: "Black"}, {piece: "Pawn", player: "Black"}, {piece: "Pawn", player: "Black"}, {piece: "Pawn", player: "Black"}, {piece: "Pawn", player: "Black"}, {piece: "Pawn", player: "Black"}, {piece: "Pawn", player: "Black"}]
  //   7 [{piece: "Rook", player: "Black"}, {piece: "Knight", player: "Black"}, {piece: "Bishop", player: "Black"}, {piece: "Queen", player: "Black"}, {piece: "King", player: "Black"}, {piece: "Bishop", player: "Black"}, {piece: "Knight", player: "Black"}, {piece: "Rook", player: "Black"}]
  // ] }

  for (const [row_index, row_obj] of game.board.rows.entries()) {
    for (const [col_index, piece] of row_obj.entries()) {
      let piece_element = get_piece(row_index, col_index);
      piece_element.className = "";
      if (piece !== null) {
        piece_element.classList.add(piece.piece.toLowerCase());
        piece_element.classList.add(piece.player.toLowerCase());
      }
    }
  }
}

window.addEventListener("DOMContentLoaded", () => {
  get_game();

  apply_to_pieces((piece, row_index, col_index) => {
    piece.addEventListener("mouseover", async (event) => {
      reset_highlighted();
      if (is_square_empty(row_index, col_index)) {
        return;
      }

      const moves = await invoke("get_possible_moves", {row: row_index, col: col_index});
      console.log(moves);

      highlight_square(row_index, col_index, "source");
      for (const move of moves) {
        const move_type = game.board.rows[move.row][move.col] == null ? "move" : "capture";
        highlight_square(move.row, move.col, move_type);
      }
    });

    piece.addEventListener("mouseout", async (event) => {
      reset_highlighted();
    });
  });

  apply_to_squares((square, row_index, col_index) => {
    square.addEventListener("click", async (event) => {
      console.log("Selecting " + row_index + "," + col_index);
      const already_selected = is_square_selected(row_index, col_index);

      if (selected !== null && !already_selected) {
        // Move
        const result = await invoke("do_move", {source_row: selected.row, source_col: selected.col, target_row: row_index, target_col: col_index});
        if (!result) {
          console.log("Invalid move");
          return;
        }

        console.log("Move done");

        selected = null;
        reset_highlighted();
        reset_highlighted("selected");
        get_game();

        return;
      }

      selected = null;
      reset_highlighted("selected");

      if (already_selected) {
        return;
      }

      if (!is_square_player(row_index, col_index, game.turn)) {
        return;
      }

      selected = {"row": row_index, "col": col_index};
      highlight_square(row_index, col_index, "selected");
    });
  });
});
