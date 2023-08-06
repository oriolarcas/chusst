import './Board.css';
import './Pieces.css';

import { Component } from 'react';
import { Row } from 'react-bootstrap';
import Badge from 'react-bootstrap/Badge';
import { invoke } from '@tauri-apps/api';

type Piece = {
  piece: string;
  player: string;
}

type SquareType = Piece | null;

type Position = {
  row: number;
  col: number;
}

type Game = {
  board: {rows: SquareType[][]};
  player: string;
  turn: number;
  last_move: Position | null;
};

type MoveDescription = {
  mv: string,
  captures: Piece[],
}

type TurnDescription = {
  white: MoveDescription,
  black: MoveDescription,
}

type Hints = Set<string>[][];

function newBoardMatrix<T>(type: {new(): T}): T[][] {
  let hints = new Array(8);
  const indices = Array.from(hints.keys());
  for (const rank of indices) {
    hints[rank] = new Array(8);
    for (const file of indices) {
      hints[rank][file] = new type();
    }
  }
  return hints;
}

function newHintMatrix(): Hints {
  return newBoardMatrix(Set<string>);
}

function newCaptureMatrix(): Position[][][] {
  return newBoardMatrix(Array<Position>);
}

enum PieceHintTypes {
  Danger = "danger",
  Advantage = "primary",
}

interface SquareProps {
  color: string,
  squareExtraClasses: Set<string>,
  piece?: Piece,
  onClick?: (event: any) => void,
  onMouseEnter?: (event: any, hint?: PieceHintTypes) => void,
  onMouseLeave?: (event: any, hint?: PieceHintTypes) => void,
  hints?: Map<PieceHintTypes, number>,
}

function Square({color, squareExtraClasses, piece, onClick, onMouseEnter, onMouseLeave, hints}: SquareProps) {
  const square_class_name = Array.from(squareExtraClasses)
    .map((class_name) => 'highlighted-' + class_name)
    .concat(['square', color])
    .join(' ');
  return <div
      className={square_class_name}
      onClick={onClick}
    >
    { piece ? (
      <div
        className={['piece', piece.piece.toLowerCase(), piece.player.toLowerCase()].join(' ')}
        onMouseEnter={onMouseEnter}
        onMouseLeave={onMouseLeave}>
        { Array.from(hints?.entries() ?? []).map(([hint_type, hint_count]) =>
            <Badge
              className='hint'
              pill
              bg={hint_type}
              onMouseOver={(event) => onMouseEnter?.(event, hint_type)}
              onMouseLeave={(event) => onMouseLeave?.(event, hint_type)}>
                {hint_count}
              </Badge>
        ) }
        </div>
    ) : null}
    </div>
    ;
}

interface BoardProps {
  onMove?: (move: string, white_captures: string[], black_captures: string[]) => void;
  onMessage?: (msg: string) => void;
}

class Board extends Component<BoardProps, {}> {
  state: {
    game?: Game,
    selected: Position | null,
    hints: Hints,
    captures: Position[][][],
  } = {
    selected: null,
    hints: newHintMatrix(),
    captures: newCaptureMatrix(),
  };

  onMessage(msg: string) {
    this.props.onMessage?.(msg);
  }

  async reloadBoard(hints?: Hints) {
    const game = await invoke('get_game');
    const captures = await invoke('get_possible_captures');

    if (hints === undefined) {
      hints = newHintMatrix();
    }
    this.setState({game, hints, captures, selected: null});
  }

  async componentDidMount() {
    await this.reloadBoard();
  }

  isSquareSelected(position: Position) {
    return this.state.selected?.row === position.row && this.state.selected?.col === position.col;
  }

  isSquareEmpty(position: Position) {
    return !(this.state.game?.board.rows[position.row][position.col]);
  }

  isSquarePlayer(position: Position, player: string) {
    return this.state.game?.board.rows[position.row][position.col]?.player === player;
  }

  async highlightPieceMoves(position: Position, selected?: boolean): Promise<Set<string>[][]> {
    let hints = newHintMatrix();

    hints[position.row][position.col].add(selected === true ? 'selected' : 'source');

    const moves: Position[] = await invoke('get_possible_moves', {row: position.row, col: position.col});

    for (const move of moves) {
      const move_type = this.state.game?.board.rows[move.row][move.col] == null ? "move" : "capture";
      hints[move.row][move.col].add(move_type);
    }

    return hints;
  }

  filterAttackingHints(hints: Hints) {
    const hints_to_remove = ['attacker'];
    for (let rank of hints) {
      for (let square of rank) {
        for (const to_remove of hints_to_remove) {
          square.delete(to_remove);
        }
      }
    }
  }

  updateHintsWithAttackers(hints: Hints, position: Position) {
    this.filterAttackingHints(hints);

    for (const attacker of this.state.captures[position.row][position.col]) {
      hints[attacker.row][attacker.col].add('attacker');
    }
  }

  onMouseEnter = async (event: any, rank: number, file: number, hint?: PieceHintTypes) => {
    let position: Position = {row: rank, col: file};
    if (hint !== undefined) {
      let hints = this.state.hints;
      this.updateHintsWithAttackers(hints, position);
      this.setState({hints});
      return;
    }

    if (this.state.selected === null && !this.isSquareEmpty(position)) {
      const hints = await this.highlightPieceMoves(position);
      this.setState({hints});
    }
  }

  onMouseLeave = (event: any, rank: number, file: number, hint?: PieceHintTypes) => {
    if (hint !== undefined) {
      let hints = this.state.hints;
      this.filterAttackingHints(hints);
      this.setState({hints});
      return;
    }

    if (this.state.selected === null) {
      let hints = newHintMatrix();
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
      const result: boolean = await invoke('do_move', {source_row: this.state.selected?.row, source_col: this.state.selected?.col, target_row: rank, target_col: file});
      if (!result) {
        console.log('Invalid move');
        return;
      }

      console.log('Move done');

      const history: TurnDescription[] = await invoke('get_history');
      const last_turn = history[history.length - 1];

      this.props.onMove?.(
        last_turn.white.mv + " " + last_turn.black.mv,
        last_turn.white.captures.map((piece) => piece.piece),
        last_turn.black.captures.map((piece) => piece.piece)
      );

      let hints = await this.highlightPieceMoves(position);

      await this.reloadBoard(hints);

      return;
    }

    if (this.state.selected === null && this.state.game?.board.rows[rank][file]?.player !== this.state.game?.player) {
      // Opponent's piece
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

          let square = this.state.game?.board.rows[rank_index][file_index];
          let piece = square?.piece;
          let player = square?.player;
          let piece_classes: Piece | undefined;

          let hints = this.state.hints[rank_index][file_index];

          if (piece && player) {
            piece_classes = {piece, player};
          }

          const attackers = this.state.captures[rank_index][file_index];
          let badges = new Map<PieceHintTypes, number>();
          if (attackers.length > 0) {
            badges.set(PieceHintTypes.Danger, attackers.length);
          }
          // badges.set(PieceHintTypes.Advantage, 1);

          return <Square
              color={bg_color}
              squareExtraClasses={hints}
              piece={piece_classes}
              onClick={(event) => this.onClick(event, rank_index, file_index)}
              onMouseEnter={(event, hint_type) => this.onMouseEnter(event, rank_index, file_index, hint_type)}
              onMouseLeave={(event, hint_type) => this.onMouseLeave(event, rank_index, file_index, hint_type)}
              hints={badges}
            />;
        })}
      </Row>
    )
  }

  render() {
    return <div className='board m-0'>
        {Array.from(Array(8).keys()).reverse().map((index) => this.rank(index))}
      </div>;
  }
}

export default Board;
