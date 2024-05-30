import './Board.css';
import './Pieces.css';

import { Component } from 'react';
import { Row } from 'react-bootstrap';
import Badge from 'react-bootstrap/Badge';
import { invoke } from '@tauri-apps/api/core';

export enum UserAction {
  Restart,
}

type Piece = {
  piece: string;
  player: string;
}

type SquareType = Piece | null;

type Position = {
  rank: number;
  file: number;
}

type Move = {
  source: Position;
  target: Position;
}

type Game = {
  board: {ranks: SquareType[][]};
  player: string;
};

export enum MateType {
  Checkmate,
  Stalemate,
}

export type PlayerGameEnd = {
  player: string;
  mate: MateType;
}

type MoveDescription = {
  mv: string,
  captures: Piece[],
  mate: string | null,
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
  onMove?: (move: string, white_captures: string[], black_captures: string[], mate: PlayerGameEnd | null) => void;
  userActionSetter?: (setter: (action: UserAction) => void) => void;
}

export class Board extends Component<BoardProps, {}> {
  state: {
    game?: Game,
    selected: Position | null,
    hints: Hints,
    captures: Position[][][],
    finished: boolean,
    choosingPromotion: Move | null,
  } = {
    selected: null,
    hints: newHintMatrix(),
    captures: newCaptureMatrix(),
    finished: false,
    choosingPromotion: null,
  };

  async reloadBoard(hints?: Hints, finished: boolean = false) {
    const game = await invoke('get_game');
    const captures = await invoke('get_possible_captures');

    if (hints === undefined) {
      hints = newHintMatrix();
    }
    this.setState({game, hints, captures, selected: null, finished, choosingPromotion: null});
  }

  onUserAction = async (action: UserAction) => {
    switch (action) {
      case UserAction.Restart:
        await invoke('restart');
        await this.reloadBoard(undefined, false);
        return;
    }
  }

  async componentDidMount() {
    await this.reloadBoard();
    this.props.userActionSetter?.(this.onUserAction);
  }

  isSquareSelected(position: Position) {
    return this.state.selected?.rank === position.rank && this.state.selected?.file === position.file;
  }

  isSquareEmpty(position: Position) {
    return !(this.state.game?.board.ranks[position.rank][position.file]);
  }

  isSquarePlayer(position: Position, player: string) {
    return this.state.game?.board.ranks[position.rank][position.file]?.player === player;
  }

  async highlightPieceMoves(position: Position, selected?: boolean): Promise<Set<string>[][]> {
    let hints = newHintMatrix();

    hints[position.rank][position.file].add(selected === true ? 'selected' : 'source');

    const moves: Position[] = await invoke('get_possible_moves', {rank: position.rank, file: position.file});

    for (const move of moves) {
      const move_type = this.state.game?.board.ranks[move.rank][move.file] == null ? "move" : "capture";
      hints[move.rank][move.file].add(move_type);
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

    for (const attacker of this.state.captures[position.rank][position.file]) {
      hints[attacker.rank][attacker.file].add('attacker');
    }
  }

  async movePiece(move: Move, promotion?: string) {
    const result: boolean = await invoke(
      'do_move',
      {
        source_rank: move.source.rank,
        source_file: move.source.file,
        target_rank: move.target.rank,
        target_file: move.target.file,
        promotion
      }
    );
    if (!result) {
      console.log('Invalid move');
      return;
    }

    console.log('Move done');

    const history: TurnDescription[] = await invoke('get_history');
    const last_turn = history[history.length - 1];
    let finished = false;

    console.log(last_turn);

    if (last_turn.black !== null) {
      let black_mate = last_turn.black.mate ? MateType[last_turn.black.mate as keyof typeof MateType] : null;

      this.props.onMove?.(
        last_turn.white.mv + " " + last_turn.black.mv,
        last_turn.white.captures.map((piece) => piece.piece),
        last_turn.black.captures.map((piece) => piece.piece),
        black_mate !== null ? {
          player: "black",
          mate: black_mate,
        } : null,
      );

      if (black_mate !== null) {
        finished = true;
      }
    } else {
      // White mate
      let white_mate = MateType[last_turn.white.mate as keyof typeof MateType];
      this.props.onMove?.(
        last_turn.white.mv,
        last_turn.white.captures.map((piece) => piece.piece),
        [],
        {
          player: "white",
          mate: white_mate,
        }
      );

      finished = true;
    }

    let hints = await this.highlightPieceMoves(move.target);

    await this.reloadBoard(hints, finished);
  }

  onMouseEnter = async (event: any, rank: number, file: number, hint?: PieceHintTypes) => {
    let position: Position = {rank, file};
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

    if (this.state.finished) {
      return;
    }

    const position: Position = {rank, file};
    const already_selected = this.isSquareSelected(position);

    if (this.state.selected !== null && !already_selected && !this.isSquarePlayer(position, this.state.game.player)) {
      const square = this.state.game.board.ranks[this.state.selected.rank][this.state.selected.file];
      const move: Move = {source: this.state.selected, target: position};

      if (square?.piece?.toLowerCase() === 'pawn' && (position.rank === 0 || position.rank === 7)) {
        // Promotion
        this.setState({choosingPromotion: move});
        return;
      }

      // Move
      await this.movePiece(move);

      return;
    }

    if (this.state.selected === null && this.state.game?.board.ranks[rank][file]?.player !== this.state.game?.player) {
      // Opponent's piece
      return;
    }

    let selected = already_selected ? null : position;
    let hints = await this.highlightPieceMoves(position, !already_selected);

    this.setState({hints, selected});
  }

  onChoosePromotion = async (piece: string) => {
    if (!this.state.choosingPromotion) {
      return;
    }
    this.movePiece(this.state.choosingPromotion, piece);
  }

  rank(rank_index: number) {
    return (
      <Row className='rank m-0'>
        {Array.from(new Array(8).keys()).map((file_index) => {
          const bg_color = this.state.finished ?
            ((file_index + rank_index) % 2 === 0 ? 'dark-endgame' : 'light-endgame') :
            ((file_index + rank_index) % 2 === 0 ? 'dark' : 'light');

          let square = this.state.game?.board.ranks[rank_index][file_index];
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

  promotionDialog() {
    if (!this.state.choosingPromotion) {
      return null;
    }
    const player = this.state.game?.player?.toLowerCase() || 'white';
    return <div className='promotion-dialog'>
      <div className='promotion-content'>
        <div className='promotion-dialog-title'>
          Choose the promotion:
        </div>
        <div className='text-center'>
          <div className={'piece knight ' + player} onClick={(event) => this.onChoosePromotion('knight')}></div>
          <div className={'piece bishop ' + player} onClick={(event) => this.onChoosePromotion('bishop')}></div>
          <div className={'piece rook ' + player} onClick={(event) => this.onChoosePromotion('rook')}></div>
          <div className={'piece queen ' + player} onClick={(event) => this.onChoosePromotion('queen')}></div>
        </div>
      </div>
    </div>;
  }

  render() {
    return <div className='board m-0'>
        {this.promotionDialog()}
        {Array.from(Array(8).keys()).reverse().map((index) => this.rank(index))}
      </div>;
  }
}

export default Board;
