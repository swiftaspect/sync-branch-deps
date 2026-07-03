//! Newline-delimited JSON — one object per event (`{"level":…,"message":…}`).
//! A generic machine-readable format any tooling can consume (a Jenkins shell
//! step, a wrapper script, a log pipeline). Selected with `--output json` /
//! `SBD_OUTPUT=json`. Deliberately *not* a findings/test format (SARIF, JUnit):
//! sbd reports progress, not code findings — see decisions/0008.

use crate::reporters::{Level, Location, Reporter};

pub struct Json;

fn level_str(level: Level) -> &'static str {
    match level {
        Level::Info => "info",
        Level::Notice => "notice",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}

impl Reporter for Json {
    fn line(&self, level: Level, msg: &str) -> String {
        // serde_json escapes the message; `preserve_order` keeps key order.
        serde_json::json!({ "level": level_str(level), "message": msg }).to_string()
    }

    fn located(&self, level: Level, loc: &Location, msg: &str) -> String {
        serde_json::json!({
            "level": level_str(level),
            "message": msg,
            "file": loc.file,
            "line": loc.line,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_one_object_per_line() {
        assert_eq!(
            Json.line(Level::Notice, "pinned x"),
            r#"{"level":"notice","message":"pinned x"}"#
        );
    }

    #[test]
    fn escapes_message() {
        let out = Json.line(Level::Error, r#"bad "quote""#);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["level"], "error");
        assert_eq!(v["message"], r#"bad "quote""#);
    }
}
