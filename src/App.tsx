import { Component } from 'react';
import './App.css';

import { Container, Row, Col } from 'react-bootstrap';

class App extends Component<{}, {}> {
  file_labels(): string[] {
    return ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
  }

  column_labels() {
    return (
      <Row className='file-labels m-0'>
      {
        this.file_labels().map((file =>
          <div className='label'>{file}</div>
        ))
      }
      </Row>
      )
  }

  rank(rank_index: number) {
    return (
      <Row className='rank m-0'>
        {this.file_labels().map((_, file_index) => {
          const bg_color = (file_index + rank_index) % 2 == 0 ? 'light' : 'dark';
          const classes = ['square', bg_color]
          return (
            <div className={classes.join(' ')}></div>
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
            {this.column_labels()}
            <div className='board m-0'>
              {Array.from(Array(8).keys()).map((index) => this.rank(index))}
            </div>
            {this.column_labels()}
          </Col>
          <Col className='px-0'></Col>
        </Row>
      </Container>
    );
  }
}

export default App;
