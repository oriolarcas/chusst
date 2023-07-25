const { invoke } = window.__TAURI__.tauri;

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

function reset_highlighted() {
  apply_to_squares((square) => square.classList.remove("highlighted"));
}

function highlight_square(row_index, col_index) {
  let square = get_square(row_index, col_index);
  square.classList.add("highlighted");
}

async function get_board() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  const board = await invoke("get_board");

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

  for (const [row_index, row_obj] of board.rows.entries()) {
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
  get_board();

  apply_to_pieces((piece, row_index, col_index) => {
    piece.addEventListener("click", async (event) => {
      const moves = await invoke("get_possible_moves", {row: row_index, col: col_index});
      console.log(moves);
      reset_highlighted();
      for (const move of moves) {
        highlight_square(move.row, move.col);
      }
    });
  });
});
