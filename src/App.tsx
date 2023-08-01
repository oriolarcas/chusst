import './App.css';
import Board from './Board';

import { Component } from 'react';
import { Container, Row, Col, FormText, FormControl } from 'react-bootstrap';
import Form from 'react-bootstrap/Form';

function FileLabels() {
  return <Row className='file-labels m-0'>
    {
      ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'].map((file =>
        <div className='label'>{file}</div>
      ))
    }
    </Row>;
}

function MessageBox() {
  return <Row>
      <Form.Control as="textarea" rows={4} disabled />
    </Row>;
}

class App extends Component<{}, {}> {

  render() {
    return (
      <Container fluid>
        <h1>Chusst</h1>
        <Row>
          <Col className='px-0'></Col>
          <Col className='p-0'>
            <FileLabels />
            <Board />
            <FileLabels />
            <MessageBox />
          </Col>
          <Col className='px-0'></Col>
        </Row>
      </Container>
    );
  }
}

export default App;
