import './App.css';
import './Pieces.css';

import Board from './Board';

import { Component, ErrorInfo, ReactNode, RefObject, createRef } from 'react';
import { Container, Row, Col } from 'react-bootstrap';
import Form from 'react-bootstrap/Form';

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

class GameRecord extends Component<{onMount: (setter: (msg: string) => void) => void}, {}> {
  state: {moves: string[]} = {
    moves: [],
  };

  addMove = (move: string) => {
    let moves = this.state.moves;
    moves.push(move);
    this.setState({moves});
  }

  componentDidMount(): void {
    this.props.onMount(this.addMove);
  }

  render(): ReactNode {
    if (this.state.moves.length === 0) {
      return null;
    }
    return <>
      <p>Game:</p>
      <ol>
      {this.state.moves.map((move) =>
        <li>{move}</li>
      )}
      </ol>
    </>
  }
}

class MessageBox extends Component<{onMount: (setter: (msg: string) => void) => void}, {}> {
  state: {messages: string[]} = {
    messages: [],
  };
  messageBoxRef: RefObject<any> = createRef();

  logMessage = (msg: string) => {
    let messages = this.state.messages;
    messages.push(msg);
    this.setState({messages});
  }

  componentDidMount(): void {
    this.props.onMount(this.logMessage);
  }

  componentDidUpdate() {
    this.messageBoxRef.current.scrollTop = this.messageBoxRef.current.scrollHeight;
  }

  render(): ReactNode {
    return <Row>
        <Form.Control as="textarea" rows={3} disabled ref={this.messageBoxRef} value={this.state.messages.join('\n')} />
      </Row>;
  }
}

class ScoreBoard extends Component<{onMount: (setter: (white_captures: string[], black_captures: string[]) => void) => void}, {}> {
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

  componentDidMount(): void {
    this.props.onMount(this.addCaptures);
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
    const black_score = black_total_score - black_total_score;

    return <Row className='score-board'>
        <Col style={{textAlign: 'left'}}>
          <p className='m-0'>White ({white_score})</p>
          <p className='captured-list'>{this.state.white_captures.map((piece) =>
            <div className={'captured piece black ' + piece.toLowerCase()}></div>
          )}</p>
        </Col>
        <Col style={{textAlign: 'right'}}>
          <p className='m-0'>Black ({black_score})</p>
          <p className='captured-list'>{this.state.black_captures.map((piece) =>
            <div className={'captured piece white ' + piece.toLowerCase()}></div>
          )}</p>
        </Col>
      </Row>;
  }
}

class App extends Component<{}, {}> {
  moveCallback?: (move: string) => void;
  captureCallback?: (white_captures: string[], black_captures: string[]) => void;
  messageLogger?: (msg: string) => void;

  onGameRecordMount = (setter: (move: string) => void) => {
    this.moveCallback = setter;
  }

  onScoreBoardMount = (setter: (white_captures: string[], black_captures: string[]) => void) => {
    this.captureCallback = setter;
  }

  onMessageBoxMount = (setter: (msg: string) => void) => {
    this.messageLogger = setter;
  }

  onMove = (move: string, white_captures: string[], black_captures: string[]) => {
    this.moveCallback?.(move);
    this.captureCallback?.(white_captures, black_captures);
  }

  onMessage = (msg: string) => {
    this.messageLogger?.(msg);
  }

  render() {
    return (
      <Container fluid>
        <h1>Chusst</h1>
        <Row>
          <Col className='px-0'>
            <Row className='file-label-row m-0' />
            <Row className='m-0'>
              <Col><GameRecord onMount={this.onGameRecordMount} /></Col>
              <Col className='m-0 p-0'><RankLabels position='left' /></Col>
            </Row>
          </Col>
          <Col className='p-0'>
            <FileLabels />
            <Board onMove={this.onMove} onMessage={this.onMessage} />
            <FileLabels />
            <ScoreBoard onMount={this.onScoreBoardMount} />
            <MessageBox onMount={this.onMessageBoxMount} />
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
