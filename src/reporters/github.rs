//! GitHub Actions workflow commands — `Warn`/`Error` become annotations that
//! surface in the checks UI; `Info`/`Notice` stay readable in the raw log.
//! Auto-selected when `GITHUB_ACTIONS=true`.

use crate::reporters::{Level, Location, Reporter};

pub struct GitHubActions;

fn command(level: Level) -> &'static str {
    match level {
        Level::Info | Level::Notice => "notice",
        Level::Warn => "warning",
        Level::Error => "error",
    }
}

impl Reporter for GitHubActions {
    fn line(&self, level: Level, msg: &str) -> String {
        match level {
            Level::Info => format!("sbd: {msg}"),
            Level::Notice => format!("::notice::{msg}"),
            Level::Warn => format!("::warning::{msg}"),
            Level::Error => format!("::error::{msg}"),
        }
    }

    fn located(&self, level: Level, loc: &Location, msg: &str) -> String {
        let mut out = format!("::{} file={}", command(level), loc.file);
        if let Some(n) = loc.line {
            out.push_str(&format!(",line={n}"));
        }
        out.push_str(&format!("::{msg}"));
        out
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

    #[test]
    fn located_emits_file_line_annotation() {
        let g = GitHubActions;
        assert_eq!(
            g.located(
                Level::Error,
                &Location {
                    file: "package.json",
                    line: Some(5)
                },
                "branch pin"
            ),
            "::error file=package.json,line=5::branch pin"
        );
        assert_eq!(
            g.located(
                Level::Warn,
                &Location {
                    file: "compose.yaml",
                    line: None
                },
                "x"
            ),
            "::warning file=compose.yaml::x"
        );
    }
}
