use anyhow::Result;
use chusst_gen::eval::{
    EngineFeedback, EngineFeedbackMessage, EngineMessage, Game, GameHistory, GameMove,
    HasStopSignal,
};
use chusst_gen::game::{BitboardGame, MoveAction};
use tokio::sync::mpsc;

use std::fmt;
use std::io::Write;

#[derive(Clone)]
pub struct GoCommand {
    pub depth: u32,
}

#[derive(Clone)]
pub struct NewGameCommand {
    pub game: Option<BitboardGame>,
    pub moves: Vec<MoveAction>,
}

#[derive(Clone)]
pub enum EngineCommand {
    NewGame(Box<NewGameCommand>), // boxed due to big size
    Go(GoCommand),
    Stop,
    Exit,
}

#[derive(Clone)]
pub enum EngineResponse {
    Ready,
    Log(String),
    Info(EngineFeedbackMessage),
    BestBranch(Option<GameMove>),
    Error(String),
}

impl fmt::Display for EngineResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let response = match self {
            EngineResponse::Ready => "Ready",
            EngineResponse::Log(_) => "Log",
            EngineResponse::Info(_) => "Info",
            EngineResponse::BestBranch(_) => "BestBranch",
            EngineResponse::Error(_) => "Error",
        };
        write!(f, "{}", response)
    }
}

struct EngineCommandReceiver<'a> {
    receiver: &'a mut mpsc::UnboundedReceiver<EngineCommand>,
    messages: Vec<EngineCommand>,
}

impl<'a> HasStopSignal for EngineCommandReceiver<'a> {
    fn stop(&mut self) -> bool {
        if let Ok(cmd) = self.receiver.try_recv() {
            match cmd {
                EngineCommand::Stop => return true,
                _ => self.messages.push(cmd),
            }
        }
        false
    }
}

struct SenderWriter {
    sender: mpsc::UnboundedSender<EngineResponse>,
}

impl std::io::Write for SenderWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8(buf.to_vec())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        let msg_len = msg.len();
        self.sender
            .send(EngineResponse::Log(msg))
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        Ok(msg_len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct BufferedSenderWriter {
    sender: mpsc::UnboundedSender<EngineResponse>,
    writer: std::io::LineWriter<SenderWriter>,
}

impl BufferedSenderWriter {
    fn new(sender: mpsc::UnboundedSender<EngineResponse>) -> BufferedSenderWriter {
        BufferedSenderWriter {
            sender: sender.clone(),
            writer: std::io::LineWriter::new(SenderWriter { sender }),
        }
    }
}

impl std::io::Write for BufferedSenderWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl BufferedSenderWriter {
    fn send(&self, msg: EngineResponse) -> Result<()> {
        self.sender.send(msg)?;
        Ok(())
    }
}

impl EngineFeedback for BufferedSenderWriter {
    fn send(&self, msg: EngineMessage) {
        match msg {
            EngineMessage::Info(msg) => {
                let _ = self.send(EngineResponse::Log(msg.message));
            }
            EngineMessage::SearchFeedback(msg) => {
                let _ = self.send(EngineResponse::Info(msg));
            }
        }
    }
}

pub async fn engine_task(
    _input: (),
    to_engine: mpsc::UnboundedReceiver<EngineCommand>,
    from_engine: mpsc::UnboundedSender<EngineResponse>,
) {
    let mut to_engine = to_engine;
    let mut communicator = BufferedSenderWriter::new(from_engine);
    let mut game = BitboardGame::new();
    let mut history = GameHistory::new();
    let mut command_receiver = EngineCommandReceiver {
        receiver: &mut to_engine,
        messages: Vec::new(),
    };

    if communicator.send(EngineResponse::Ready).is_err() {
        return;
    }

    loop {
        let command_option = command_receiver.messages.pop();
        let command = command_option.or(command_receiver.receiver.recv().await);
        match command {
            Some(EngineCommand::NewGame(new_game_cmd)) => {
                if let Some(new_game) = new_game_cmd.game {
                    game = new_game;
                    history.clear();
                }
                for mv in new_game_cmd.moves {
                    if game.do_move(&mv).is_none() {
                        let _ = communicator
                            .send(EngineResponse::Error(format!("Invalid move {}", mv.mv)));
                    }
                    history.push(mv);
                }
            }
            Some(EngineCommand::Go(go_command)) => {
                let best_move = game.get_best_move_with_logger(
                    go_command.depth,
                    &history,
                    &mut command_receiver,
                    &mut communicator,
                );
                let _ignore_error = communicator.send(EngineResponse::BestBranch(Some(best_move)));
            }
            Some(EngineCommand::Stop) => (),
            Some(EngineCommand::Exit) => break,
            None => {
                let _ = writeln!(communicator, "Broken command pipeline");
                break;
            }
        }
    }
}
