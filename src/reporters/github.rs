//! GitHub Actions workflow commands — `Warn`/`Error` become annotations that
//! surface in the checks UI; `Info`/`Notice` stay readable in the raw log.
//! Auto-selected when `GITHUB_ACTIONS=true`.

use crate::reporters::{Level, Reporter};

pub struct GitHubActions;

impl Reporter for GitHubActions {
    fn line(&self, level: Level, msg: &str) -> String {
        match level {
            Level::Info => format!("sbd: {msg}"),
            Level::Notice => format!("::notice::{msg}"),
            Level::Warn => format!("::warning::{msg}"),
            Level::Error => format!("::error::{msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_workflow_commands() {
        let g = GitHubActions;
        assert_eq!(g.line(Level::Info, "hi"), "sbd: hi");
        assert_eq!(g.line(Level::Notice, "pinned"), "::notice::pinned");
        assert_eq!(g.line(Level::Warn, "careful"), "::warning::careful");
        assert_eq!(g.line(Level::Error, "boom"), "::error::boom");
    }
}
