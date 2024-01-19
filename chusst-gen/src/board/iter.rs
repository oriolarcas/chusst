use super::{ModifiableBoard, Piece, PieceType, Player, Position};
use crate::pos;
use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub struct Direction {
    pub row_inc: i8,
    pub col_inc: i8,
}

pub fn try_move(position: &Position, direction: &Direction) -> Option<Position> {
    let row = i8::try_from(position.rank).unwrap() + direction.row_inc;
    let col = i8::try_from(position.file).unwrap() + direction.col_inc;

    if !(0..8).contains(&row) {
        return None;
    }

    if !(0..8).contains(&col) {
        return None;
    }

    match (usize::try_from(row), usize::try_from(col)) {
        (Ok(urow), Ok(ucol)) => Some(pos!(urow, ucol)),
        _ => None,
    }
}

pub trait PositionIterator<'a, R>: Iterator<Item = Position> + Clone + Sized {
    fn representation(&self) -> &R;

    fn only_empty(&self) -> IterateEmpty<'a, Self, R> {
        IterateEmpty::new(self.clone())
    }

    fn only_player(&self, player: Player) -> IterateByPlayer<'a, Self, R> {
        IterateByPlayer::new(self.clone(), player)
    }

    fn only_enemy(&self, player: Player) -> IterateByPlayer<'a, Self, R> {
        IterateByPlayer::new(self.clone(), !player)
    }

    fn only_piece(&self, piece: PieceType) -> IterateByPiece<'a, Self, R> {
        IterateByPiece::new(self.clone(), piece)
    }

    fn only_enemy_piece(
        &self,
        player: Player,
        piece: PieceType,
    ) -> IterateByPlayerPiece<'a, Self, R> {
        IterateByPlayerPiece::new(self.clone(), !player, piece)
    }

    fn only_empty_or_enemy(&self, player: Player) -> IteratePlayerOrEmpty<'a, Self, R> {
        IteratePlayerOrEmpty::new(self.clone(), !player)
    }

    fn take_while_empty(&self) -> IterateWhileEmpty<'a, Self, R> {
        IterateWhileEmpty::new(self.clone())
    }

    fn take_while_empty_or_enemy(&self, player: Player) -> IterateWhileEmptyOrPlayer<'a, Self, R> {
        IterateWhileEmptyOrPlayer::new(self.clone(), !player)
    }

    fn skip_while_empty(&self) -> IterateSkipWhileEmpty<'a, Self, R> {
        IterateSkipWhileEmpty::new(self.clone())
    }
}

// Generators:

// PositionIter: iterator over a single position

pub struct PositionIter<'a, R> {
    representation: &'a R,
    position: Option<Position>,
}

impl<'a, R> PositionIterator<'a, R> for PositionIter<'a, R> {
    fn representation(&self) -> &R {
        self.representation
    }
}

impl<'a, R> Clone for PositionIter<'a, R> {
    fn clone(&self) -> Self {
        Self {
            representation: self.representation,
            position: self.position,
        }
    }
}

impl<'a, R> Iterator for PositionIter<'a, R> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        let current_position = self.position;
        self.position = None;
        current_position
    }
}

impl<'a, R> PositionIter<'a, R> {
    pub fn new(representation: &'a R, position: Option<Position>) -> Self {
        PositionIter {
            representation,
            position,
        }
    }
}

// DirectionIter: iterator over the positions in a direction

pub struct DirectionIter<'a, R> {
    representation: &'a R,
    position: Option<Position>,
    direction: Direction,
}

impl<'a, R> PositionIterator<'a, R> for DirectionIter<'a, R> {
    fn representation(&self) -> &R {
        self.representation
    }
}

impl<'a, R> Clone for DirectionIter<'a, R> {
    fn clone(&self) -> Self {
        Self {
            representation: self.representation,
            position: self.position,
            direction: self.direction,
        }
    }
}

impl<'a, R> Iterator for DirectionIter<'a, R> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        self.position = self
            .position
            .and_then(|position| try_move(&position, &self.direction));
        self.position
    }
}

impl<'a, R> DirectionIter<'a, R> {
    pub fn new(representation: &'a R, position: Option<Position>, direction: Direction) -> Self {
        DirectionIter {
            representation,
            position,
            direction,
        }
    }
}

// BoardIter: iterator over all the positions of a board

pub struct BoardIter<'a, R> {
    representation: &'a R,
    position: Position,
}

impl<'a, R> PositionIterator<'a, R> for BoardIter<'a, R> {
    fn representation(&self) -> &R {
        self.representation
    }
}

impl<'a, R> Clone for BoardIter<'a, R> {
    fn clone(&self) -> Self {
        Self {
            representation: self.representation,
            position: self.position,
        }
    }
}

impl<'a, R> Iterator for BoardIter<'a, R> {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position.rank > 7 {
            return None;
        }

        let current_position = self.position;

        if self.position.file == 7 {
            self.position.rank += 1;
            self.position.file = 0;
        } else {
            self.position.file += 1;
        }

        Some(current_position)
    }
}

impl<'a, R> BoardIter<'a, R> {
    pub fn new(representation: &'a R) -> Self {
        BoardIter {
            representation,
            position: Position { rank: 0, file: 0 },
        }
    }
}

// Filters:

// IterateEmpty

pub struct IterateEmpty<'a, I, R> {
    iter: I,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IterateEmpty<'a, I, R>
where
    I: PositionIterator<'a, R> + Clone,
{
    pub fn new(iter: I) -> Self {
        IterateEmpty {
            iter,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IterateEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IterateEmpty<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IterateEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let position = self.iter.next()?;
            if self.iter.representation().at(&position).is_none() {
                return Some(position);
            }
        }
    }
}

// IterateByPlayer

pub struct IterateByPlayer<'a, I, R> {
    iter: I,
    player: Player,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IterateByPlayer<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    pub fn new(iter: I, player: Player) -> Self {
        IterateByPlayer {
            iter,
            player,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IterateByPlayer<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>> + 'a,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IterateByPlayer<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            player: self.player,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IterateByPlayer<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let position = self.iter.next()?;
            match self.iter.representation().at(&position) {
                Some(piece) if piece.player == self.player => return Some(position),
                _ => continue,
            }
        }
    }
}

// IterateByPiece

pub struct IterateByPiece<'a, I, R> {
    iter: I,
    piece: PieceType,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IterateByPiece<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    pub fn new(iter: I, piece: PieceType) -> Self {
        IterateByPiece {
            iter,
            piece,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IterateByPiece<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IterateByPiece<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            piece: self.piece,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IterateByPiece<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let position = self.iter.next()?;
            match self.iter.representation().at(&position) {
                Some(piece) if piece.piece == self.piece => return Some(position),
                _ => continue,
            }
        }
    }
}

// IterateByPlayerPiece

pub struct IterateByPlayerPiece<'a, I, R> {
    iter: I,
    player: Player,
    piece: PieceType,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IterateByPlayerPiece<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    pub fn new(iter: I, player: Player, piece: PieceType) -> Self {
        IterateByPlayerPiece {
            iter,
            player,
            piece,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IterateByPlayerPiece<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IterateByPlayerPiece<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            player: self.player,
            piece: self.piece,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IterateByPlayerPiece<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let position = self.iter.next()?;
            match self.iter.representation().at(&position) {
                Some(piece) if piece.piece == self.piece && piece.player == self.player => {
                    return Some(position)
                }
                _ => continue,
            }
        }
    }
}

// IteratePlayerOrEmpty

pub struct IteratePlayerOrEmpty<'a, I, R> {
    iter: I,
    player: Player,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IteratePlayerOrEmpty<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    pub fn new(iter: I, player: Player) -> Self {
        IteratePlayerOrEmpty {
            iter,
            player,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IteratePlayerOrEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IteratePlayerOrEmpty<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            player: self.player,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IteratePlayerOrEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let position = self.iter.next()?;
            match self.iter.representation().at(&position) {
                Some(piece) if piece.player == self.player => return Some(position),
                None => return Some(position),
                _ => continue,
            }
        }
    }
}

// IterateWhileEmpty

pub struct IterateWhileEmpty<'a, I, R> {
    iter: I,
    stop: bool,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IterateWhileEmpty<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    pub fn new(iter: I) -> Self {
        IterateWhileEmpty {
            iter,
            stop: false,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IterateWhileEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IterateWhileEmpty<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            stop: self.stop,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IterateWhileEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop {
            return None;
        }

        if let Some(position) = self.iter.next() {
            if self.iter.representation().at(&position).is_none() {
                return Some(position);
            }
        }

        self.stop = true;
        None
    }
}

// IterateWhileEmptyOrPlayer

pub struct IterateWhileEmptyOrPlayer<'a, I, R> {
    iter: I,
    player: Player,
    stop: bool,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IterateWhileEmptyOrPlayer<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    pub fn new(iter: I, player: Player) -> Self {
        IterateWhileEmptyOrPlayer {
            iter,
            player,
            stop: false,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IterateWhileEmptyOrPlayer<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IterateWhileEmptyOrPlayer<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            player: self.player,
            stop: self.stop,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IterateWhileEmptyOrPlayer<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop {
            return None;
        }

        if let Some(position) = self.iter.next() {
            if self.iter.representation().at(&position).is_none() {
                return Some(position);
            }
        }

        self.stop = true;
        None
    }
}

// IterateSkipEmpty

pub struct IterateSkipWhileEmpty<'a, I, R> {
    iter: I,
    _unused: PhantomData<&'a R>,
}

impl<'a, I, R> IterateSkipWhileEmpty<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    pub fn new(iter: I) -> Self {
        IterateSkipWhileEmpty {
            iter,
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> PositionIterator<'a, R> for IterateSkipWhileEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    fn representation(&self) -> &R {
        self.iter.representation()
    }
}

impl<'a, I, R> Clone for IterateSkipWhileEmpty<'a, I, R>
where
    I: PositionIterator<'a, R>,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            _unused: PhantomData,
        }
    }
}

impl<'a, I, R> Iterator for IterateSkipWhileEmpty<'a, I, R>
where
    R: ModifiableBoard<Position, Option<Piece>>,
    I: PositionIterator<'a, R>,
{
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let position = self.iter.next()?;
            if self.iter.representation().at(&position).is_some() {
                return Some(position);
            }
        }
    }
}
