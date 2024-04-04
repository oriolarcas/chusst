use anyhow::{anyhow, Result};
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use std::io::stdin;
use std::os::fd::{FromRawFd, RawFd};
use tokio::sync::mpsc;

pub type StdinResponse = Result<String>;

pub async fn stdin_task(
    poll: Poll,
    _: mpsc::UnboundedReceiver<()>,
    stdin_lines_sender: mpsc::UnboundedSender<StdinResponse>,
) -> anyhow::Result<()> {
    const STDIN_FILENO: i32 = 0;
    const STDIN_TOKEN: Token = Token(0);
    let stdin_file: RawFd = unsafe { FromRawFd::from_raw_fd(STDIN_FILENO) };
    let mut stdin_fd = SourceFd(&stdin_file);
    let mut events = Events::with_capacity(1);

    let mut poll = poll;
    poll.registry()
        .register(&mut stdin_fd, STDIN_TOKEN, Interest::READABLE)?;

    let stdin = stdin();

    'main_loop: loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                STDIN_TOKEN => {
                    let mut line = String::new();
                    let message = match stdin.read_line(&mut line) {
                        Ok(_) => Ok(line),
                        Err(err) => Err(anyhow!(err)),
                    };
                    stdin_lines_sender.send(message)?;
                }
                // Waker
                _ => break 'main_loop,
            }
        }
    }

    Ok(())
}
