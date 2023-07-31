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

class App extends Component<{}, {}> {
  state: {game: Game | null} = {game: null};

  constructor(props: any) {
    super(props);
  }

  async reloadBoard() {
    const game = await invoke('get_game');
    this.setState({game});
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

  rank(rank_index: number) {
    return (
      <Row className='rank m-0'>
        {Array.from(new Array(8).keys()).map((file_index) => {
          const bg_color = (file_index + rank_index) % 2 == 0 ? 'dark' : 'light';

          let square_classes = ['square', bg_color];
          let square = this.state.game?.board.rows[rank_index][file_index];
          let piece = square?.piece;
          let player = square?.player;
          let piece_classes = ['piece'];

          if (piece && player) {
            piece_classes.push(piece.toLowerCase());
            piece_classes.push(player.toLowerCase());
          }

          return (
            <div className={square_classes.join(' ')}>
              <div className={piece_classes.join(' ')}></div>
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
