mod duplex_thread;
mod engine;
mod stdin;

use anyhow::Result;
use chusst_gen::eval::GameMove;
use chusst_gen::game::{BitboardGame, ModifiableGame, MoveAction};
use duplex_thread::DuplexChannel;
use engine::{EngineCommand, EngineResponse, GoCommand, NewGameCommand};
use mio::{Poll, Token, Waker};
use rust_fsm::*;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use stdin::{stdin_task, StdinResponse};

use crate::duplex_thread::create_duplex_thread;
use crate::engine::engine_task;

enum SyncInput {
    FromStdin(StdinResponse),
    FromEngine(EngineResponse),
}

struct Logger {
    file: File,
    last_event: std::time::Instant,
}

impl Logger {
    pub fn update(&mut self) -> f64 {
        let now = Instant::now();
        let elapsed = self.last_event.elapsed();
        self.last_event = now;
        (elapsed.as_micros() as f64) / 1000.
    }
}

state_machine! {
    // derive(Debug)
    UciProtocol(Initializing)

    Initializing(CommandUci) => Ready [OutputCommandId],
    Ready => {
        CommandGo => Searching [EngineCommandGo],
        CommandPosition => Ready [EngineCommandPosition],
        // Additional commands
        CommandUciNewGame => Ready [EngineCommandNewGame],
        CommandIsReady => Ready [OutputCommandReadyOk],
        CommandSetOption => Ready [EngineCommandSetOption],
        CommandStop => Ready [OutputSavedCommandBestMove],
    },
    Searching => {
        CommandStop => WaitingForResult [EngineCommandStop],
        EngineInfo => Searching [OutputCommandInfo],
        EngineResult => Ready [SaveBestMove],
        // Additional commands
        CommandIsReady => Ready [OutputCommandReadyOk],
    },
    WaitingForResult => {
        EngineInfo => WaitingForResult [OutputCommandInfo],
        EngineResult => Ready [OutputCommandBestMove],
        // Additional commands
        CommandIsReady => Ready [OutputCommandReadyOk],
        CommandStop => WaitingForResult,
    },
}

enum ParsedInput {
    UciStdInInput(Vec<String>),
    EngineMessage(EngineResponse),
}

const SEARCH_DEPTH_DEFAULT: u32 = 4;
const SEARCH_DEPTH_MIN: u32 = 2;
const SEARCH_DEPTH_MAX: u32 = 5;

impl fmt::Display for UciProtocolState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = match self {
            UciProtocolState::Initializing => "Initializing",
            UciProtocolState::Ready => "Ready",
            UciProtocolState::Searching => "Searching",
            UciProtocolState::WaitingForResult => "WaitingForResult",
        };
        write!(f, "{}", state)
    }
}

async fn uci_main() -> Result<()> {
    let mut logger = Logger {
        file: match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("uci.{}.log", std::process::id()))
        {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Could not open the log file: {}", err);
                return Ok(());
            }
        },
        last_event: Instant::now(),
    };

    const WAKE_TOKEN: Token = Token(10);
    let poll = Poll::new()?;
    let waker = Waker::new(poll.registry(), WAKE_TOKEN)?;

    let mut engine_thread = create_duplex_thread("Engine", engine_task, ());
    let mut stdin_thread = create_duplex_thread("Stdin", stdin_task, poll);

    uci_loop(
        &mut logger,
        &mut stdin_thread.channel,
        &mut engine_thread.channel,
    )
    .await;

    // Wake stdin task
    if let Err(err) = waker.wake() {
        let _ = writeln!(logger.file, "Error waking up stdin task: {}", err);
    }

    // Stop engine
    let _ = engine_thread.channel.to_thread.send(EngineCommand::Stop);
    let _ = engine_thread.channel.to_thread.send(EngineCommand::Exit);

    if let Err(_stdin_err) = stdin_thread.thread_handle.join() {
        let _ = writeln!(logger.file, "stdin task finished with error");
    }
    if let Err(_engine_err) = engine_thread.thread_handle.join() {
        let _ = writeln!(logger.file, "Engine task with error");
    }

    Ok(())
}

async fn uci_loop(
    logger: &mut Logger,
    stdin_channel: &mut DuplexChannel<(), StdinResponse>,
    engine_channel: &mut DuplexChannel<EngineCommand, EngineResponse>,
) {
    macro_rules! write_command {
        ($str:expr) => {
            {
                let elapsed = logger.update();
                let _ = writeln!(logger.file, "< +{} {}", elapsed, $str);
                println!($str);
            }
        };
        ($fmt:expr, $($param:expr),*) => {
            {
                let elapsed = logger.update();
                let _ = writeln!(logger.file, "< +{} {}", elapsed, format!($fmt, $($param),+));
                println!($fmt, $($param),+);
            }
        };
    }

    macro_rules! log {
        ($str:expr) => {
            {
                let _ = writeln!(logger.file, $str);
            }
        };
        ($fmt:expr, $($param:expr),*) => {
            {
                let _ = writeln!(logger.file, $fmt, $($param),+);
            }
        };
    }

    let mut search_depth = 3;

    log!("Starting engine");

    match engine_channel.from_thread.recv().await {
        Some(EngineResponse::Ready) => (),
        _ => {
            log!("Could not start engine thread");
            return;
        }
    }

    log!("Engine ready");

    let mut uci_protocol: StateMachine<UciProtocol> = StateMachine::new();

    let mut last_best_move: Option<Option<GameMove>> = None;

    loop {
        let input = tokio::select! {
            Some(stdin_line) = stdin_channel.from_thread.recv() => Some(SyncInput::FromStdin(stdin_line)),
            Some(engine_response) = engine_channel.from_thread.recv() => Some(SyncInput::FromEngine(engine_response)),
            else => None,
        };

        let (protocol_input, parsed_input) = match &input {
            Some(SyncInput::FromStdin(stdin_response)) => match &stdin_response {
                Ok(stdin_line) => {
                    let words = parse_command(stdin_line.as_str(), logger);
                    let protocol_stdin_input = if let Some(stdin_command) = words.first() {
                        match stdin_command.as_str() {
                            "uci" => UciProtocolInput::CommandUci,
                            "isready" => UciProtocolInput::CommandIsReady,
                            "setoption" => UciProtocolInput::CommandSetOption,
                            "ucinewgame" => UciProtocolInput::CommandUciNewGame,
                            "position" => UciProtocolInput::CommandPosition,
                            "go" => UciProtocolInput::CommandGo,
                            "stop" => UciProtocolInput::CommandStop,
                            // Exit
                            "quit" => break,
                            // Unknown command, ignore:
                            _ => {
                                log!("Unknown command");
                                continue;
                            }
                        }
                    } else {
                        continue;
                    };
                    (protocol_stdin_input, ParsedInput::UciStdInInput(words))
                }
                Err(error_message) => {
                    log!("stdin error: {error_message}");
                    log!("Exiting");
                    break;
                }
            },
            Some(SyncInput::FromEngine(engine_response)) => {
                let engine_protocol_input = match &engine_response {
                    EngineResponse::Log(message) => {
                        let trimmed_message = message.trim();
                        if !trimmed_message.is_empty() {
                            log!("(engine) {}", message.trim());
                        }
                        continue;
                    }
                    EngineResponse::Ready => continue,
                    EngineResponse::Info(_) => UciProtocolInput::EngineInfo,
                    EngineResponse::BestBranch(_) => UciProtocolInput::EngineResult,
                    EngineResponse::Error(error) => {
                        log!("{}", error);
                        break;
                    }
                };
                (
                    engine_protocol_input,
                    ParsedInput::EngineMessage(engine_response.clone()),
                )
            }
            None => {
                log!("I/O error: input channels have been closed unexpectedly");
                log!("Exiting");
                break;
            }
        };

        let previous_state = uci_protocol.state().to_string();

        let protocol_output_result = uci_protocol.consume(&protocol_input);

        let protocol_output = match &protocol_output_result {
            Ok(output) => output,
            Err(_) => {
                log!("Unexpected input, ignoring");
                continue;
            }
        };

        let current_state = uci_protocol.state().to_string();

        if current_state != previous_state {
            log!(
                "New state: {} -> {}",
                previous_state,
                uci_protocol.state().to_string()
            );
        }

        match (protocol_output, parsed_input) {
            (Some(UciProtocolOutput::OutputCommandId), _) => {
                write_command!("id name Chusst {}", env!("CARGO_PKG_VERSION"));
                write_command!(
                    "option name SearchDepth type spin default {} min {} max {}",
                    SEARCH_DEPTH_DEFAULT,
                    SEARCH_DEPTH_MIN,
                    SEARCH_DEPTH_MAX
                );
                write_command!("uciok");
            }
            (
                Some(UciProtocolOutput::EngineCommandSetOption),
                ParsedInput::UciStdInInput(words),
            ) => {
                if let Some(&["name", name, "value", value]) = words
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<&str>>()
                    .get(1..5)
                {
                    match name {
                        "SearchDepth" => {
                            if let Ok(int_value) = value.parse::<u32>() {
                                match int_value {
                                    SEARCH_DEPTH_MIN..=SEARCH_DEPTH_MAX => search_depth = int_value,
                                    _ => log!("SearchDepth value out of range"),
                                }
                            } else {
                                log!("Invalid SearchDepth value");
                            }
                        }
                        _ => log!("Unknown parameter"),
                    }
                } else {
                    log!("Malformed setparam command");
                }
            }
            (Some(UciProtocolOutput::OutputCommandReadyOk), ParsedInput::UciStdInInput(_)) => {
                write_command!("readyok")
            }
            (Some(UciProtocolOutput::EngineCommandNewGame), ParsedInput::UciStdInInput(_)) => {
                if engine_channel
                    .to_thread
                    .send(EngineCommand::NewGame(NewGameCommand {
                        game: Some(BitboardGame::new()),
                        moves: Vec::new(),
                    }))
                    .is_err()
                {
                    log!("Error: could not send new game to engine");
                    break;
                }
            }
            (Some(UciProtocolOutput::EngineCommandPosition), ParsedInput::UciStdInInput(words)) => {
                let mut param_iter = words.iter().skip(1).map(String::as_str);
                let (next_token, new_game) = match param_iter.next() {
                    Some("startpos") => (param_iter.next(), Some(BitboardGame::new())),
                    Some("fen") => {
                        log!("Parsing FEN string...");

                        let fen: Vec<&str> = [
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                        ]
                        .iter()
                        .map_while(|token| *token)
                        .collect();

                        if let Some(new_game_from_fen) = BitboardGame::try_from_fen(fen.as_slice())
                        {
                            (param_iter.next(), Some(new_game_from_fen))
                        } else {
                            log!("Malformed FEN string in position command");
                            continue;
                        }
                    }
                    Some(token) => (Some(token), None),
                    _ => {
                        log!("Malformed position command");
                        continue;
                    }
                };
                if let Some(game) = &new_game {
                    log!("New position:\n{}", game.board());
                }
                let mut new_game_command = NewGameCommand {
                    game: new_game,
                    moves: Vec::new(),
                };
                match next_token {
                    Some("moves") => {
                        for mv_str in param_iter {
                            if let Some(mv) = MoveAction::try_from_long_algebraic_str(mv_str) {
                                new_game_command.moves.push(mv);
                            } else {
                                log!("Malformed move in position command");
                                continue;
                            }
                        }
                    }
                    Some(_) => {
                        log!("Malformed move in position command");
                        continue;
                    }
                    None => (),
                }
                if engine_channel
                    .to_thread
                    .send(EngineCommand::NewGame(new_game_command))
                    .is_err()
                {
                    log!("Error: could not send new game command to engine");
                    break;
                }
            }
            (Some(UciProtocolOutput::EngineCommandGo), ParsedInput::UciStdInInput(words)) => {
                // go infinite
                //   Search until stop is received
                // go wtime 300000 btime 300000 movestogo 40
                //   Search with this amount of time
                match words.get(1).map(String::as_str) {
                    Some("infinite") => {
                        if engine_channel
                            .to_thread
                            .send(EngineCommand::Go(GoCommand {
                                depth: search_depth,
                            }))
                            .is_err()
                        {
                            log!("Error: could not send go command to engine");
                            break;
                        }
                    }
                    _ => {
                        log!("Error: Unknown go command");
                        break;
                    }
                }
            }
            (
                Some(UciProtocolOutput::SaveBestMove),
                ParsedInput::EngineMessage(EngineResponse::BestBranch(best_move_result)),
            ) => {
                let best_move_str = move_to_uci_string(&best_move_result);
                write_command!("bestmove {}", best_move_str);
                last_best_move = Some(best_move_result);
            }
            (Some(UciProtocolOutput::EngineCommandStop), ParsedInput::UciStdInInput(_)) => {
                if engine_channel.to_thread.send(EngineCommand::Stop).is_err() {
                    log!("Error: could not send go command to engine");
                    break;
                }
            }
            (
                Some(UciProtocolOutput::OutputSavedCommandBestMove),
                ParsedInput::UciStdInInput(_),
            ) => {
                if let Some(best_move) = &last_best_move {
                    let best_move_str = move_to_uci_string(best_move);
                    write_command!("bestmove {}", best_move_str);
                }
            }
            (
                Some(UciProtocolOutput::OutputCommandInfo),
                ParsedInput::EngineMessage(EngineResponse::Info(info)),
            ) => {
                write_command!(
                    "info depth {} nodes {} score cp {}",
                    info.depth,
                    info.nodes,
                    info.score
                );
            }
            (
                Some(UciProtocolOutput::OutputCommandBestMove),
                ParsedInput::EngineMessage(EngineResponse::BestBranch(best_move_result)),
            ) => {
                let best_move_str = move_to_uci_string(&best_move_result);
                write_command!("bestmove {}", best_move_str);
            }

            // No action
            (None, _) => continue,
            // Should never come to this
            _ => {
                log!("Unexpected protocol state");
                break;
            }
        }
    }
}

fn move_to_uci_string(mv: &Option<GameMove>) -> String {
    match mv {
        Some(GameMove::Normal(best_move)) => {
            format!("{}{}", best_move.mv.source, best_move.mv.target)
        }
        Some(GameMove::Mate(_)) | None => "0000".to_owned(),
    }
}

fn parse_command(line: &str, logger: &mut Logger) -> Vec<String> {
    let trimmed_buffer = line.trim();
    let elapsed = logger.update();
    let _result = writeln!(logger.file, "> +{} {}", elapsed, trimmed_buffer);
    trimmed_buffer
        .split_whitespace()
        .map(String::from)
        .collect()
}

fn main() -> Result<()> {
    #[cfg(feature = "tokio-console")]
    console_subscriber::init();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(uci_main())?;

    Ok(())
}
