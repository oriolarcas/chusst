use anyhow::bail;
use anyhow::{Error as AnyhowError, Result};
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{alpha1, char, digit1, multispace1, one_of};
use nom::combinator::recognize;
use nom::error::Error as NomError;
use nom::multi::many1;
use nom::sequence::{delimited, separated_pair, terminated};
use nom::IResult;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(PartialEq)]
enum PgnSection {
    Nothing,
    Header,
    Movetext,
}

#[derive(PartialEq)]
pub enum Color {
    White,
    Black,
}

trait ErrorExplainer<I, O> {
    fn explain(self, context: &str, line: u32) -> Result<(I, O), AnyhowError>;
}

impl<I, O> ErrorExplainer<I, O> for IResult<I, O, NomError<I>> {
    fn explain(self, context: &str, line: u32) -> Result<(I, O), AnyhowError> {
        match self {
            Ok(ok) => Ok(ok),
            Err(_) => {
                bail!("{} (line {})", context, line,);
            }
        }
    }
}

fn pgn_tag(input: &str) -> IResult<&str, (&str, &str)> {
    delimited(
        char('['),
        separated_pair(
            alpha1,
            multispace1,
            delimited(char('"'), is_not("\""), char('"')),
        ),
        char(']'),
    )(input)
}

fn san_move(input: &str) -> IResult<&str, &str> {
    recognize(many1(one_of("abcdefghKQRBNOx=+#-12345678")))(input)
}

fn white_move_number(input: &str) -> IResult<&str, &str> {
    terminated(digit1, char('.'))(input)
}

fn black_move_number(input: &str) -> IResult<&str, &str> {
    terminated(digit1, tag("..."))(input)
}

fn result(input: &str) -> IResult<&str, &str> {
    alt((tag("1-0"), tag("0-1"), tag("1/2-1/2"), tag("*")))(input)
}

pub trait LexerVisitor {
    fn parse_file(&mut self, path: &PathBuf) -> Result<()> {
        let file = File::open(path)?;
        let lines = BufReader::new(file).lines();
        let mut line_number = 0u32;

        let mut section = PgnSection::Nothing;
        for line_result in lines {
            line_number += 1;
            let line = line_result?;
            let mut input = line.as_str();

            loop {
                input = input.trim_start();

                if input.is_empty() {
                    break;
                }

                (input, section) = match section {
                    PgnSection::Nothing => {
                        let (_input, _token) =
                            char('[')(input).explain("Expected game header", line_number)?;

                        self.begin_game()?;
                        self.begin_header()?;

                        (input, PgnSection::Header)
                    }
                    PgnSection::Header => match pgn_tag(input) {
                        Ok((input, (name, value))) => {
                            self.tag(name, value)?;
                            (input, PgnSection::Header)
                        }
                        Err(_) => {
                            self.end_header()?;
                            self.begin_movetext()?;

                            (input, PgnSection::Movetext)
                        }
                    },
                    PgnSection::Movetext => {
                        if let Ok((input, number)) = black_move_number(input) {
                            self.move_number(number, Color::Black)?;
                            (input, PgnSection::Movetext)
                        } else if let Ok((input, number)) = white_move_number(input) {
                            self.move_number(number, Color::White)?;
                            (input, PgnSection::Movetext)
                        } else if let Ok((input, mv)) = san_move(input) {
                            self.san_move(mv)?;
                            (input, PgnSection::Movetext)
                        } else if let Ok((input, result_str)) = result(input) {
                            self.result(result_str)?;
                            self.end_movetext()?;
                            self.end_game()?;
                            (input, PgnSection::Nothing)
                        } else if let Ok((_input, _)) = char::<&str, NomError<&str>>('[')(input) {
                            self.end_movetext()?;
                            self.end_game()?;
                            (input, PgnSection::Nothing)
                        } else {
                            bail!("Unexpected token (at line {}): '{}'", line_number, input);
                        }
                    }
                }
            }
        }

        match section {
            PgnSection::Nothing => (),
            PgnSection::Header => bail!("Incomplete game data (at line {})", line_number),
            PgnSection::Movetext => {
                self.end_movetext()?;
                self.end_game()?;
            }
        }

        Ok(())
    }

    fn begin_game(&mut self) -> Result<()>;
    fn begin_header(&mut self) -> Result<()>;
    fn tag(&mut self, name: &str, value: &str) -> Result<()>;
    fn end_header(&mut self) -> Result<()>;
    fn begin_movetext(&mut self) -> Result<()>;
    fn move_number(&mut self, number: &str, color: Color) -> Result<()>;
    fn san_move(&mut self, mv: &str) -> Result<()>;
    fn begin_comment(&mut self) -> Result<()>;
    fn comment_data(&mut self, data: &str) -> Result<()>;
    fn end_comment(&mut self) -> Result<()>;
    fn begin_variation(&mut self) -> Result<()>;
    fn end_variation(&mut self) -> Result<()>;
    fn result(&mut self, result: &str) -> Result<()>;
    fn end_movetext(&mut self) -> Result<()>;
    fn end_game(&mut self) -> Result<()>;
}
