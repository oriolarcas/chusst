import './App.css';
import './Pieces.css';

import {Board, PlayerGameEnd, MateType, UserAction} from './Board';

import { Component, ReactNode, RefObject, createRef } from 'react';
import { Container, Row, Col } from 'react-bootstrap';
import Button from 'react-bootstrap/Button';

function FileLabels() {
  return <Row className='file-label-row file-labels m-0'>
    {
      ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'].map((file =>
        <div className='label'>{file}</div>
      ))
    }
    </Row>;
}

function RankLabels(props: {position: string}) {
  return <>
    {
      Array.from(new Array(8).keys()).reverse().map((file =>
        <Row className={'rank-labels ' + props.position + ' m-0 p-0'}>
          <div className='label'>{file + 1}</div>
        </Row>
      ))
    }
  </>;
}

class GameRecord extends Component<{
  onMount: (
    onMoveSetter: (msg: string, mate: PlayerGameEnd | null) => void,
    onRestartSetter: () => void,
  ) => void,
  onUpdate: () => void,
}, {}> {
  state: {moves: string[], mate?: string} = {
    moves: [],
  };

  addMove = (move: string, mate: PlayerGameEnd | null) => {
    let moves = this.state.moves;
    moves.push(move);
    let mate_state = undefined;
    if (mate) {
      if (mate.mate === MateType.Checkmate) {
        switch (mate.player) {
          case "white":
            mate_state = "White wins";
            break;
          case "black":
            mate_state = "Black wins";
            break;
        }
      } else if (mate.mate === MateType.Stalemate) {
        mate_state = "Stalemate";
      }
    }
    this.setState({moves, mate: mate_state});
  }

  onRestart = () => {
    this.setState({moves: []});
  }

  componentDidMount(): void {
    this.props.onMount(this.addMove, this.onRestart);
  }

  componentDidUpdate(): void {
    this.props.onUpdate();
  }

  endgameLine() {
    if (this.state.mate) {
      return <p>{this.state.mate}</p>
    }
    return null;
  }

  render(): ReactNode {
    if (this.state.moves.length === 0) {
      return null;
    }
    return <>
      <ol>
      {this.state.moves.map((move) =>
        <li>{move}</li>
      )}
      </ol>
      {this.endgameLine()}
    </>
  }
}

class ScoreBoard extends Component<{
  onMount: (
    captureCallback: (white_captures: string[], black_captures: string[]) => void,
    restartCallback: () => void,
  ) => void,
}, {}> {
  state: {white_captures: string[], black_captures: string[]} = {
    white_captures: [],
    black_captures: [],
  }

  addCaptures = (new_white_captures: string[], new_black_captures: string[]) => {
    let white_captures = this.state.white_captures;
    let black_captures = this.state.black_captures;
    white_captures.push(...new_white_captures);
    black_captures.push(...new_black_captures);
    this.setState({white_captures, black_captures})
  }

  onRestart = () => {
    this.setState({white_captures: [], black_captures: [],});
  }

  componentDidMount(): void {
    this.props.onMount(this.addCaptures, this.onRestart);
  }

  getPieceScore(piece: string): number {
    switch (piece.toLowerCase()) {
      case "pawn":   return 1;
      case "knight": return 3;
      case "bishop": return 3;
      case "rook":   return 5;
      case "queen":  return 9;
    }
    return 0;
  }

  getPlayerScore(captures: string[]): number {
    return captures.map((piece) => this.getPieceScore(piece)).reduce((total, piece_value) => total + piece_value, 0);
  }

  render(): ReactNode {
    const white_total_score = this.getPlayerScore(this.state.white_captures);
    const black_total_score = this.getPlayerScore(this.state.black_captures);

    const white_score = white_total_score - black_total_score;
    const black_score = black_total_score - white_total_score;

    // Hide pawns captured by both players
    const count_pawns = (captured: string[]) => captured.filter((piece) => piece.toLowerCase() == "pawn").length;
    const white_captured_pawns = count_pawns(this.state.white_captures);
    const black_captured_pawns = count_pawns(this.state.black_captures);
    const common_captured_pawns = Math.min(white_captured_pawns, black_captured_pawns);
    const pawn_filter = function* (captured: string[]) {
      let counter = 0;
      for (const piece of captured) {
        if (piece.toLowerCase() == "pawn") {
          counter++;
          if (counter <= common_captured_pawns) {
            continue;
          }
        }
        yield piece;
      }
    };

    const filtered_white_captures = Array.from(pawn_filter(this.state.white_captures));
    const filtered_black_captures = Array.from(pawn_filter(this.state.black_captures));

    return <Row className='score-board'>
        <Col style={{textAlign: 'left'}}>
          <p className='m-0'>White ({white_score})</p>
          <p className='captured-list'>{filtered_white_captures.map((piece) =>
            <div className={'captured piece black ' + piece.toLowerCase()}></div>
          )}</p>
        </Col>
        <Col style={{textAlign: 'right'}}>
          <p className='m-0'>Black ({black_score})</p>
          <p className='captured-list'>{filtered_black_captures.map((piece) =>
            <div className={'captured piece white ' + piece.toLowerCase()}></div>
          )}</p>
        </Col>
      </Row>;
  }
}

class App extends Component<{}, {}> {
  state: {finished: boolean} = {finished: false};
  moveCallback?: (move: string, mate: PlayerGameEnd | null) => void;
  recordRestartCallback?: () => void;
  captureCallback?: (white_captures: string[], black_captures: string[]) => void;
  scoreboardRestartCallback?: () => void;
  gameStatusChangeCallback?: (finished: boolean) => void;
  messageLogger?: (msg: string) => void;
  gameHistoryRef?: RefObject<HTMLDivElement> = createRef();
  userActionCallback?: (action: UserAction) => void;

  onGameRecordMount = (moveCallback: (move: string, mate: PlayerGameEnd | null) => void, restartCallback: () => void) => {
    this.moveCallback = moveCallback;
    this.recordRestartCallback = restartCallback;
  }

  onScoreBoardMount = (captureCallback: (white_captures: string[], black_captures: string[]) => void, restartCallback: () => void) => {
    this.captureCallback = captureCallback;
    this.scoreboardRestartCallback = restartCallback;
  }

  onOptionBoxMount = (callback: (finished: boolean) => void) => {
    this.gameStatusChangeCallback = callback;
  }

  onMove = (move: string, white_captures: string[], black_captures: string[], mate: PlayerGameEnd | null) => {
    this.moveCallback?.(move, mate);
    this.captureCallback?.(white_captures, black_captures);
    if (mate !== null) {
      this.setState({finished: true});
    }
  }

  onGameRecordUpdate = () => {
    let gameRecord = this.gameHistoryRef?.current;
    if (gameRecord) {
      gameRecord.scrollTop = Math.pow(10, 10);
    }
  }

  onUserActionCallbackSet = (setter: (action: UserAction) => void) => {
    this.userActionCallback = setter;
  }

  onRestart() {
    this.userActionCallback?.(UserAction.Restart);
    this.recordRestartCallback?.();
    this.scoreboardRestartCallback?.();
    this.setState({finished: false});
  }

  render() {
    return (
      <Container fluid>
        <h1>Chusst</h1>
        <Row>
          <Col className='px-0'>
            <Row className='file-label-row m-0' />
            <Row className='m-0'>
              <Col style={{height: "480px", overflowY: "auto"}} ref={this.gameHistoryRef}>
                <GameRecord onMount={this.onGameRecordMount} onUpdate={this.onGameRecordUpdate} />
              </Col>
              <Col className='col-md-auto m-0 p-0'><RankLabels position='left' /></Col>
            </Row>
          </Col>
          <Col className='p-0'>
            <FileLabels />
            <Board onMove={this.onMove} userActionSetter={this.onUserActionCallbackSet} />
            <FileLabels />
            <ScoreBoard onMount={this.onScoreBoardMount} />
            <Row>
              <Button variant={this.state.finished ? "primary" : "secondary"} onClick={(ev) => this.onRestart()}>Restart</Button>
            </Row>
          </Col>
          <Col className='px-0'>
            <Row className='file-label-row m-0' />
            <RankLabels position='right' />
          </Col>
        </Row>
      </Container>
    );
  }
}

export default App;
