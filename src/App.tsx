import './App.css';
import Board from './Board';

import { Component, RefObject, createRef } from 'react';
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
      Array.from(new Array(8).keys()).map((file =>
        <Row className={'rank-labels ' + props.position + ' m-0'}>
          <div className='label'>{file}</div>
        </Row>
      ))
    }
  </>;
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

  render() {
    return <Row>
        <Form.Control as="textarea" rows={4} disabled ref={this.messageBoxRef} value={this.state.messages.join('\n')} />
      </Row>;
  }
}

class App extends Component<{}, {}> {
  messageLogger?: (msg: string) => void;

  onMessageBoxMount = (setter: (msg: string) => void) => {
    this.messageLogger = setter;
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
            <Row className='file-label-row' />
            <RankLabels position='left' />
          </Col>
          <Col className='p-0'>
            <FileLabels />
            <Board onMessage={this.onMessage} />
            <FileLabels />
            <MessageBox onMount={this.onMessageBoxMount} />
          </Col>
          <Col className='px-0'>
            <Row className='file-label-row' />
            <RankLabels position='right' />
          </Col>
        </Row>
      </Container>
    );
  }
}

export default App;
