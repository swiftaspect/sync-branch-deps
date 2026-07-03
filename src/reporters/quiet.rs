//! Quiet mode: suppress routine progress (`info`/`notice`) and surface only
//! `warn`/`error`. "No news is good news" — useful in scripts and cron where you
//! only care whether something needs attention. Selected with `--output quiet`
//! / `SBD_OUTPUT=quiet`. (Exit status still signals failure regardless.)

use crate::reporters::{Level, Reporter};

pub struct Quiet;

impl Reporter for Quiet {
    fn line(&self, level: Level, msg: &str) -> String {
        match level {
            Level::Warn => format!("sbd: warning: {msg}"),
            Level::Error => format!("sbd: error: {msg}"),
            // Suppressed: an empty line is the "nothing to print" sentinel.
            Level::Info | Level::Notice => String::new(),
        }
    }

    fn report(&self, level: Level, msg: &str) {
        let line = self.line(level, msg);
        if !line.is_empty() {
            eprintln!("{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppresses_progress_shows_problems() {
        assert_eq!(Quiet.line(Level::Info, "resolving"), "");
        assert_eq!(Quiet.line(Level::Notice, "pinned x"), "");
        assert_eq!(Quiet.line(Level::Warn, "careful"), "sbd: warning: careful");
        assert_eq!(Quiet.line(Level::Error, "boom"), "sbd: error: boom");
    }
}
