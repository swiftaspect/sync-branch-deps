//! Human-readable output, optionally colorized. The default outside CI.

use crate::reporters::{Level, Reporter};

pub struct Plain {
    color: bool,
}

impl Plain {
    pub fn new(color: bool) -> Self {
        Self { color }
    }

    fn paint(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
}

impl Reporter for Plain {
    fn line(&self, level: Level, msg: &str) -> String {
        match level {
            Level::Info => format!("sbd: {msg}"),
            Level::Notice => format!("sbd: {}", self.paint("32", msg)), // green
            Level::Warn => format!("sbd: {}", self.paint("33", &format!("warning: {msg}"))),
            Level::Error => format!("sbd: {}", self.paint("31", &format!("error: {msg}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_color_is_prefixed_and_bare() {
        let p = Plain::new(false);
        assert_eq!(p.line(Level::Info, "hi"), "sbd: hi");
        assert_eq!(p.line(Level::Warn, "careful"), "sbd: warning: careful");
        assert_eq!(p.line(Level::Error, "boom"), "sbd: error: boom");
        assert!(!p.line(Level::Notice, "ok").contains('\x1b'));
    }

    #[test]
    fn with_color_wraps_in_ansi() {
        let p = Plain::new(true);
        assert!(p.line(Level::Notice, "ok").contains("\x1b[32m"));
    }
}
