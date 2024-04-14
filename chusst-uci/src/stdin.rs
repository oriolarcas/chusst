use std::os::fd::{AsRawFd, FromRawFd, RawFd};

use anyhow::{anyhow, Result};
use libc::STDIN_FILENO;
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use tokio::sync::mpsc;

pub type StdinResponse = Result<String>;

/// Reads from stdin and sends lines to the `stdin_lines_sender`.
/// `poll` is used to poll stdin for readability. It contains a waker that is used to wake up the
/// task when the program is exiting.
/// Rust does not support non-blocking stdin reads, so we have to use `poll` to poll stdin for
/// readability and then read from stdin using `read(2)` with `O_NONBLOCK` set.
pub async fn stdin_task(
    poll: Poll,
    _: mpsc::UnboundedReceiver<()>,
    stdin_lines_sender: mpsc::UnboundedSender<StdinResponse>,
) -> anyhow::Result<()> {
    make_stdin_nonblocking()?;

    const STDIN_TOKEN: Token = Token(0);
    let stdin_file: RawFd = unsafe { FromRawFd::from_raw_fd(STDIN_FILENO) };
    let mut stdin_fd = SourceFd(&stdin_file);
    let mut events = Events::with_capacity(1);

    let mut poll = poll;
    poll.registry()
        .register(&mut stdin_fd, STDIN_TOKEN, Interest::READABLE)?;

    let mut stdin_buffer = String::new();

    'main_loop: loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                STDIN_TOKEN => {
                    // Consume stdin completely using read(2)
                    loop {
                        let mut buf = [0; 1024];
                        let read_result = unsafe {
                            libc::read(STDIN_FILENO, buf.as_mut_ptr() as *mut _, buf.len())
                        };
                        match read_result {
                            -1 => {
                                if let Some(err) = std::io::Error::last_os_error().raw_os_error() {
                                    if err == libc::EWOULDBLOCK || err == libc::EAGAIN {
                                        break;
                                    }
                                }
                                return Err(anyhow!("read(stdin) failed"));
                            }
                            0 => {
                                break;
                            }
                            _ => {
                                let read_size = read_result as usize;
                                stdin_buffer.push_str(
                                    String::from_utf8_lossy(&buf[..read_size])
                                        .to_string()
                                        .as_str(),
                                );
                                let mut lines = LinesWithRemainder::new(&stdin_buffer);
                                for line in &mut lines {
                                    stdin_lines_sender.send(Ok(line.to_string()))?;
                                }
                                if let Some(remaining) = lines.remainder() {
                                    stdin_buffer = remaining.to_string();
                                } else {
                                    stdin_buffer.clear();
                                }
                            }
                        }
                    }
                }
                // Waker
                _ => break 'main_loop,
            }
        }
    }

    Ok(())
}

fn make_stdin_nonblocking() -> Result<()> {
    let stdin_fd = std::io::stdin().as_raw_fd();
    let flags = unsafe { libc::fcntl(stdin_fd, libc::F_GETFL) };
    if flags == -1 {
        return Err(anyhow!("fcntl(stdin, F_GETFL) failed"));
    }

    let new_flags = flags | libc::O_NONBLOCK;
    if unsafe { libc::fcntl(stdin_fd, libc::F_SETFL, new_flags) } == -1 {
        return Err(anyhow!("fcntl(stdin, F_SETFL) failed"));
    }

    Ok(())
}

/// A reimplementation of [`std::str::Lines`] that will only iterate over complete lines
/// with a line ending. After iterating, an incomplete line can be retrieved using the
/// [`remainder`](LinesWithRemainder::remainder) method.
struct LinesWithRemainder<'a> {
    string: &'a str,
    pos: usize,
}

impl LinesWithRemainder<'_> {
    fn new(string: &str) -> LinesWithRemainder {
        LinesWithRemainder { string, pos: 0 }
    }

    fn remainder(&self) -> Option<&str> {
        if self.pos >= self.string.len() {
            None
        } else {
            Some(&self.string[self.pos..])
        }
    }
}

impl<'a> Iterator for LinesWithRemainder<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.string.len() {
            return None;
        }

        let Some(line_ending_pos) = self.string[self.pos..].find('\n') else {
            return None;
        };
        let next_line = &self.string[self.pos..self.pos + line_ending_pos];
        // Skip the newline character
        self.pos += next_line.len() + 1;
        // Skip the carriage return character if it exists
        if Some("\r") == self.string.get(self.pos..self.pos + 1) {
            self.pos += 1;
        }
        Some(next_line)
    }
}
