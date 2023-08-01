import './App.css';

import { Component } from 'react';
import { Container, Row, Col } from 'react-bootstrap';
import { invoke } from '@tauri-apps/api';

type Square = {
  piece: string;
  player: string;
} | null;

type Position = {
  row: number;
  col: number;
}

type Game = {
  board: {rows: Square[][]};
  player: string;
  turn: number;
  last_move: Position | null;
};

type Hints = Set<string>[][];

function createHints(): Hints {
  let hints = new Array(8);
  const indices = Array.from(hints.keys());
  for (const rank of indices) {
    hints[rank] = new Array(8);
    for (const file of indices) {
      hints[rank][file] = new Set();
    }
  }
  return hints;
}

class App extends Component<{}, {}> {
  state: {
    game: Game | null,
    selected: Position | null,
    hints: Hints,
  } = {
    game: null,
    selected: null,
    hints: createHints(),
  };

  async reloadBoard(hints: Hints | null = null) {
    const game = await invoke('get_game');
    if (hints === null) {
      hints = createHints();
    }
    this.setState({game, hints, selected: null});
  }

  async componentDidMount() {
    await this.reloadBoard();
  }

  columnLabels() {
    return (
      <Row className='file-labels m-0'>
      {
        ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'].map((file =>
          <div className='label'>{file}</div>
        ))
      }
      </Row>
      )
  }

  isSquareSelected(position: Position) {
    return this.state.selected?.row === position.row && this.state.selected?.col === position.col;
  }

  isSquareEmpty(position: Position) {
    return !(this.state.game?.board.rows[position.row][position.col]);
  }

  isSquarePlayer(position: Position, player: string) {
    return this.state.game?.board.rows[position.row][position.col]?.player == player;
  }

  async highlightPieceMoves(position: Position, selected: boolean = false): Promise<Set<string>[][]> {
    let hints = createHints();

    hints[position.row][position.col].add(selected ? 'selected' : 'source');

    const moves: Position[] = await invoke('get_possible_moves', {row: position.row, col: position.col});

    for (const move of moves) {
      const move_type = this.state.game?.board.rows[move.row][move.col] == null ? "move" : "capture";
      hints[move.row][move.col].add(move_type);
    }

    return hints;
  }

  onMouseOver = async (event: any, rank: number, file: number) => {
    let position: Position = {row: rank, col: file};
    if (this.state.selected === null && !this.isSquareEmpty(position)) {
      const hints = await this.highlightPieceMoves(position);
      this.setState({hints});
    }
  }

  onMouseOut = (event: any, rank: number, file: number) => {
    if (this.state.selected === null) {
      let hints = createHints();
      this.setState({hints});
    }
  }

  onClick = async (event: any, rank: number, file: number) => {
    if (!this.state.game) {
      return;
    }

    const position: Position = {row: rank, col: file};
    const already_selected = this.isSquareSelected(position);

    if (this.state.selected !== null && !already_selected && !this.isSquarePlayer(position, this.state.game.player)) {
      // Move
      const result = await invoke('do_move', {source_row: this.state.selected.row, source_col: this.state.selected.col, target_row: rank, target_col: file});
      if (!result) {
        console.log('Invalid move');
        return;
      }

      console.log('Move done');

      let hints = await this.highlightPieceMoves(position);

      await this.reloadBoard(hints);

      return;
    }

    let selected = already_selected ? null : position;
    let hints = await this.highlightPieceMoves(position, !already_selected);

    this.setState({hints, selected});
  }

  rank(rank_index: number) {
    return (
      <Row className='rank m-0'>
        {Array.from(new Array(8).keys()).map((file_index) => {
          const bg_color = (file_index + rank_index) % 2 === 0 ? 'dark' : 'light';

          let square_classes = ['square', bg_color];
          let square = this.state.game?.board.rows[rank_index][file_index];
          let piece = square?.piece;
          let player = square?.player;
          let piece_classes = ['piece'];

          let hints = this.state.hints?.[rank_index][file_index];
          if (hints) {
            square_classes = square_classes.concat(Array.from(hints).map((class_name) => 'highlighted-' + class_name));
          }

          if (piece && player) {
            piece_classes.push(piece.toLowerCase());
            piece_classes.push(player.toLowerCase());
          }

          return (
            <div className={square_classes.join(' ')}>
              <div
                className={piece_classes.join(' ')}
                onClick={(event) => this.onClick(event, rank_index, file_index)}
                onMouseOver={(event) => this.onMouseOver(event, rank_index, file_index)}
                onMouseOut={(event) => this.onMouseOut(event, rank_index, file_index)}></div>
            </div>
          )
        })}
      </Row>
    )
  }

  render() {
    return (
      <Container fluid>
        <h1>Chusst</h1>
        <Row>
          <Col className='px-0'></Col>
          <Col className='p-0'>
            {this.columnLabels()}
            <div className='board m-0'>
              {Array.from(Array(8).keys()).reverse().map((index) => this.rank(index))}
            </div>
            {this.columnLabels()}
          </Col>
          <Col className='px-0'></Col>
        </Row>
      </Container>
    );
  }
}

export default App;
