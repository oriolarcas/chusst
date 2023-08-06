import './App.css';
import Board from './Board';

import { Component, ReactNode, RefObject, createRef } from 'react';
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

class ScoreBoard extends Component<{}, {}> {
  render(): ReactNode {
    return <Row>
        <Col style={{textAlign: 'left'}}>
          White
        </Col>
        <Col style={{textAlign: 'right'}}>
          Black
        </Col>
      </Row>;
  }
}

class App extends Component<{}, {}> {
  moveCallback?: (msg: string) => void;
  messageLogger?: (msg: string) => void;

  onMessageBoxMount = (setter: (msg: string) => void) => {
    this.messageLogger = setter;
  }

  onGameRecordMount = (setter: (move: string) => void) => {
    this.moveCallback = setter;
  }

  onMove = (move: string) => {
    this.moveCallback?.(move);
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
            <ScoreBoard />
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
