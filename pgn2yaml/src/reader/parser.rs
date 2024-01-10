use std::path::PathBuf;

use crate::reader::lexer::{Color, LexerVisitor};
use anyhow::{bail, Context, Result};

pub struct Move {
    pub white: String,
    pub black: Option<String>,
}

#[derive(Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Default)]
pub struct PGN {
    pub tags: Vec<Tag>,
    pub moves: Vec<Move>,
    pub result: String,
}

#[derive(Default)]
pub struct Parser {
    pgns: Vec<PGN>,
    current: Option<PGN>,
    variation_level: u32,
}

impl Parser {
    pub fn parse_file(path: &PathBuf) -> Result<Vec<PGN>> {
        let mut parser = Parser::default();
        parser.parse_file(path)?;
        Ok(parser.pgns)
    }

    fn current(&self) -> Result<&PGN> {
        self.current.as_ref().context("No current game")
    }

    fn current_mut(&mut self) -> Result<&mut PGN> {
        self.current.as_mut().context("No current game")
    }

    fn save_current_game(&mut self) -> Result<()> {
        if let Some(current) = self.current.take() {
            self.pgns.push(current);
        }
        Ok(())
    }
}

impl LexerVisitor for Parser {
    fn begin_game(&mut self) -> Result<()> {
        if self.current.is_some() {
            bail!("Unexpected new game");
        }

        self.current = Some(PGN::default());
        Ok(())
    }

    fn begin_header(&mut self) -> Result<()> {
        Ok(())
    }

    fn tag(&mut self, name: &str, value: &str) -> Result<()> {
        let tag = Tag {
            key: name.to_string(),
            value: value.to_string(),
        };
        self.current_mut()?.tags.push(tag);
        Ok(())
    }

    fn end_header(&mut self) -> Result<()> {
        Ok(())
    }

    fn begin_movetext(&mut self) -> Result<()> {
        Ok(())
    }

    fn move_number(&mut self, number: &str, color: Color) -> Result<()> {
        if self.variation_level > 0 {
            return Ok(());
        }

        let parsed_number = number.parse::<usize>()?;

        if color == Color::White {
            let expected = self.current()?.moves.len() + 1;
            if expected != parsed_number {
                bail!("Unexpected move number {}, expected {}", parsed_number, expected);
            }
        } else {
            let expected = self.current()?.moves.len();
            if expected != parsed_number {
                bail!("Unexpected move number {}, expected {}", parsed_number, expected);
            }
        }

        Ok(())
    }

    fn san_move(&mut self, mv: &str) -> Result<()> {
        if self.variation_level > 0 {
            // Ignore variations
            return Ok(());
        }

        let current = self.current_mut()?;
        if current.moves.is_empty() {
            current.moves.push(Move {
                white: mv.to_string(),
                black: None,
            });
        } else {
            let last_move = current.moves.last_mut().unwrap();
            if last_move.black.is_some() {
                current.moves.push(Move {
                    white: mv.to_string(),
                    black: None,
                });
            } else {
                last_move.black = Some(mv.to_string());
            }
        }
        Ok(())
    }

    fn begin_comment(&mut self) -> Result<()> {
        Ok(())
    }

    fn comment_data(&mut self, _data: &str) -> Result<()> {
        Ok(())
    }

    fn end_comment(&mut self) -> Result<()> {
        Ok(())
    }

    fn begin_variation(&mut self) -> Result<()> {
        self.variation_level += 1;
        Ok(())
    }

    fn end_variation(&mut self) -> Result<()> {
        if self.variation_level == 0 {
            bail!("Unexpected end of variation");
        }
        self.variation_level -= 1;
        Ok(())
    }

    fn result(&mut self, result: &str) -> Result<()> {
        self.current_mut()?.result = result.to_string();
        Ok(())
    }

    fn end_movetext(&mut self) -> Result<()> {
        if self.variation_level > 0 {
            bail!("Unexpected end of movetext");
        }
        if self.current()?.moves.is_empty() {
            bail!("Unexpected end of game");
        }
        Ok(())
    }

    fn end_game(&mut self) -> Result<()> {
        if self.variation_level > 0 {
            bail!("Unexpected end of game");
        }
        if self.current()?.moves.is_empty() {
            bail!("Unexpected end of game");
        }

        self.save_current_game()?;
        Ok(())
    }
}
