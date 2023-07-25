const { invoke } = window.__TAURI__.tauri;

let greetInputEl;
let greetMsgEl;

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

  console.log(board);

  const col_id_map = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
  for (const [row_index, row_obj] of board.rows.entries()) {
    for (const [col_index, square] of row_obj.entries()) {
      const row_id = row_index + 1;
      const col_id = col_id_map[col_index];
      console.log(col_id + row_id + ": " + square);
      let square_element = document.querySelector(".row-" + row_id + " .col-" + col_id + " div");
      square_element.className = "";
      if (square !== null) {
        square_element.classList.add(square.piece.toLowerCase());
        square_element.classList.add(square.player.toLowerCase());
      }
    }
  }
}

window.addEventListener("DOMContentLoaded", () => {
  // boardEl = document.querySelector("#board");
  get_board();
});
