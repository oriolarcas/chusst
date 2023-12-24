#[derive(Clone)]
pub struct EngineFeedbackMessage {
    pub depth: u32, // in plies
    pub nodes: u32,
    pub score: i32, // in centipawns
}

pub trait EngineFeedback: std::io::Write {
    fn send(&self, msg: EngineFeedbackMessage);
}

pub trait SearchFeedback: std::io::Write {
    fn update(self: &mut Self, depth: u32, nodes: u32, score: i32);
}

#[derive(Default)]
pub struct SilentSearchFeedback();

impl SearchFeedback for SilentSearchFeedback {
    fn update(self: &mut Self, _depth: u32, _nodes: u32, _score: i32) {
        // do nothing
    }
}

impl std::io::Write for SilentSearchFeedback {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct PeriodicalSearchFeedback<'a> {
    update_interval: std::time::Duration,
    last_update: std::time::Instant,
    receiver: &'a mut dyn EngineFeedback,
}

impl<'a> PeriodicalSearchFeedback<'a> {
    pub fn new(
        update_interval: std::time::Duration,
        receiver: &'a mut impl EngineFeedback,
    ) -> Self {
        PeriodicalSearchFeedback {
            update_interval,
            last_update: std::time::Instant::now(),
            receiver,
        }
    }
}

impl<'a> SearchFeedback for PeriodicalSearchFeedback<'a> {
    fn update(self: &mut Self, depth: u32, nodes: u32, score: i32) {
        let now = std::time::Instant::now();

        if now - self.last_update < self.update_interval {
            return;
        }

        self.receiver.send(EngineFeedbackMessage {
            depth,
            nodes,
            score,
        });

        self.last_update = now;
    }
}

impl<'a> std::io::Write for PeriodicalSearchFeedback<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.receiver.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.receiver.flush()
    }
}

#[derive(Default)]
pub struct StdoutFeedback();

impl EngineFeedback for StdoutFeedback {
    fn send(&self, _msg: EngineFeedbackMessage) {
        // ignore
    }
}

impl std::io::Write for StdoutFeedback {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        std::io::stdout().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}
