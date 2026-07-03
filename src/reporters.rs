//! Output formatting. sbd emits progress, not data, so everything goes to
//! stderr — but *how* it's formatted adapts to where it runs: a human terminal,
//! a plain pipe, or a CI system that understands log commands. Selection is
//! automatic (env-detected) or forced via `--output` / `SBD_OUTPUT`.
//!
//! Adding a format (GitLab, TeamCity, JSON, …) is a new file under `reporters/`
//! implementing [`Reporter`], plus a match arm in [`select`].

use std::io::IsTerminal;

pub mod github;
pub mod json;
pub mod plain;
pub mod quiet;

/// Severity of a reported line; formats render each differently (e.g. a CI
/// reporter turns `Warn`/`Error` into annotations).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Notice,
    Warn,
    Error,
}

pub trait Reporter {
    /// Render a line for `level`/`msg` (pure — no I/O, so it's unit-testable).
    fn line(&self, level: Level, msg: &str) -> String;

    fn report(&self, level: Level, msg: &str) {
        eprintln!("{}", self.line(level, msg));
    }
    fn info(&self, msg: &str) {
        self.report(Level::Info, msg);
    }
    fn notice(&self, msg: &str) {
        self.report(Level::Notice, msg);
    }
    fn warn(&self, msg: &str) {
        self.report(Level::Warn, msg);
    }
    fn error(&self, msg: &str) {
        self.report(Level::Error, msg);
    }
}

/// Choose a reporter from an explicit choice (`--output`/`SBD_OUTPUT`), or for
/// `auto`/unknown/none, by detecting the environment.
pub fn select(choice: Option<&str>) -> Box<dyn Reporter> {
    match choice {
        Some("plain") => Box::new(plain::Plain::new(false)),
        Some("color") => Box::new(plain::Plain::new(true)),
        Some("github") => Box::new(github::GitHubActions),
        Some("json") => Box::new(json::Json),
        Some("quiet") => Box::new(quiet::Quiet),
        _ => auto(),
    }
}

fn auto() -> Box<dyn Reporter> {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        Box::new(github::GitHubActions)
    } else {
        // Color only when writing to a terminal and NO_COLOR is unset.
        let color = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
        Box::new(plain::Plain::new(color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_choice_overrides_detection() {
        assert_eq!(
            select(Some("github")).line(Level::Warn, "x"),
            "::warning::x"
        );
        assert_eq!(
            select(Some("plain")).line(Level::Warn, "x"),
            "sbd: warning: x"
        );
    }
}
