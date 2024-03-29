use crate::duplex_thread::{create_duplex_thread, DuplexThread};
use chusst_gen::eval::{
    EngineFeedback, EngineFeedbackMessage, EngineMessage, Game, GameMove, HasStopSignal,
};
use chusst_gen::game::{BitboardGame, MoveAction};

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
    NewGame(NewGameCommand),
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
    receiver: &'a crossbeam_channel::Receiver<EngineCommand>,
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
    sender: std::rc::Rc<crossbeam_channel::Sender<EngineResponse>>,
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
    sender: std::rc::Rc<crossbeam_channel::Sender<EngineResponse>>,
    writer: std::io::LineWriter<SenderWriter>,
}

impl BufferedSenderWriter {
    fn new(sender: crossbeam_channel::Sender<EngineResponse>) -> BufferedSenderWriter {
        let sender_rc = std::rc::Rc::new(sender);
        BufferedSenderWriter {
            sender: sender_rc.clone(),
            writer: std::io::LineWriter::new(SenderWriter { sender: sender_rc }),
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
    fn send(
        &self,
        msg: EngineResponse,
    ) -> Result<(), crossbeam_channel::SendError<EngineResponse>> {
        self.sender.send(msg)
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

fn engine_thread(
    to_engine: crossbeam_channel::Receiver<EngineCommand>,
    from_engine: crossbeam_channel::Sender<EngineResponse>,
) {
    let mut communicator = BufferedSenderWriter::new(from_engine);
    let mut game = BitboardGame::new();
    let mut command_receiver = EngineCommandReceiver {
        receiver: &to_engine,
        messages: Vec::new(),
    };

    if communicator.send(EngineResponse::Ready).is_err() {
        return;
    }

    loop {
        let command = command_receiver.messages.pop().or(to_engine.recv().ok());
        match command {
            Some(EngineCommand::NewGame(new_game_cmd)) => {
                if let Some(new_game) = new_game_cmd.game {
                    game = new_game;
                }
                for mv in new_game_cmd.moves {
                    if game.do_move(&mv).is_none() {
                        let _ = communicator
                            .send(EngineResponse::Error(format!("Invalid move {}", mv.mv)));
                    }
                }
            }
            Some(EngineCommand::Go(go_command)) => {
                let best_move = game.get_best_move_with_logger(
                    go_command.depth,
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

pub fn create_engine_thread<'scope>(
    scope: &'scope std::thread::Scope<'scope, '_>,
) -> DuplexThread<'scope, EngineCommand, EngineResponse> {
    create_duplex_thread(scope, engine_thread)
}
