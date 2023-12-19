use crate::duplex_thread::{create_duplex_thread, DuplexThread};
use chusst::game::{Game, Move};
use chusst::moves::{
    do_move, get_best_move_with_logger, EngineFeedback, EngineFeedbackMessage, GameMove,
    HasStopSignal,
};

use std::io::Write;
use std::fmt;

#[derive(Clone)]
pub struct GoCommand {
    pub depth: u32,
}

#[derive(Clone)]
pub struct NewGameCommand {
    pub game: Option<Game>,
    pub moves: Vec<Move>,
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

struct SenderWriter<'a> {
    sender: std::rc::Rc<&'a mut crossbeam_channel::Sender<EngineResponse>>,
}

impl<'a> std::io::Write for SenderWriter<'a> {
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

struct BufferedSenderWriter<'a> {
    sender: std::rc::Rc<&'a mut crossbeam_channel::Sender<EngineResponse>>,
    writer: std::io::LineWriter<SenderWriter<'a>>,
}

impl<'a> BufferedSenderWriter<'a> {
    fn new(sender: &'a mut crossbeam_channel::Sender<EngineResponse>) -> BufferedSenderWriter<'a> {
        let sender_rc = std::rc::Rc::new(sender);

        BufferedSenderWriter {
            sender: sender_rc.clone(),
            writer: std::io::LineWriter::new(SenderWriter {
                sender: sender_rc.clone(),
            }),
        }
    }
}

impl<'a> std::io::Write for BufferedSenderWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<'a> BufferedSenderWriter<'a> {
    fn send(
        &self,
        msg: EngineResponse,
    ) -> Result<(), crossbeam_channel::SendError<EngineResponse>> {
        self.sender.send(msg)
    }
}

impl<'a> EngineFeedback for BufferedSenderWriter<'a> {
    fn send(&self, msg: EngineFeedbackMessage) {
        let _ = self.send(EngineResponse::Info(msg));
    }
}

fn engine_thread(
    to_engine: crossbeam_channel::Receiver<EngineCommand>,
    from_engine: crossbeam_channel::Sender<EngineResponse>,
) {
    let mut from_engine_mut = from_engine;
    let mut communicator = BufferedSenderWriter::new(&mut from_engine_mut);
    let mut game = Game::new();
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
                    if do_move(&mut game, &mv).is_none() {
                        let _ = communicator
                            .send(EngineResponse::Error(format!("Invalid move {}", mv)));
                    }
                }
            }
            Some(EngineCommand::Go(go_command)) => {
                let best_move = get_best_move_with_logger(
                    &mut game,
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

pub fn create_engine_thread<'scope, 'env>(
    scope: &'scope std::thread::Scope<'scope, 'env>,
) -> DuplexThread<'scope, EngineCommand, EngineResponse> {
    create_duplex_thread(scope, engine_thread)
}
