//! Minimal argument parsing for `sbd <sync|verify> [flags]`. The surface is
//! small — two subcommands plus a couple of flags — so it's hand-rolled to keep
//! the binary dependency-light. `parse` is pure and unit-tested.

pub const USAGE: &str = "\
sbd — resolve and pin cross-repo feature-branch dependencies

Usage:
  sbd sync [--dry-run] [--output <fmt>]    Resolve branch artifacts and pin them
  sbd verify [--output <fmt>]              Fail if any branch pin remains (the PR gate)

Options:
  --dry-run          (sync) report what would be pinned without writing
  --output <fmt>     plain | color | github | json | quiet  (default: auto-detect)
  -h, --help         show this help";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Sync { dry_run: bool },
    Verify,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub output: Option<String>,
}

/// The outcome of parsing argv (excluding the program name).
#[derive(Debug, PartialEq, Eq)]
pub enum Parsed {
    /// A valid invocation to run.
    Run(Invocation),
    /// `-h`/`--help`: print usage to stdout, exit 0.
    Help,
    /// Invalid usage: print the message + usage to stderr, exit non-zero.
    Usage(String),
}

pub fn parse(args: &[String]) -> Parsed {
    let mut iter = args.iter();
    let Some(first) = iter.next() else {
        return Parsed::Usage("a subcommand is required".into());
    };

    let is_sync = match first.as_str() {
        "-h" | "--help" => return Parsed::Help,
        "sync" => true,
        "verify" => false,
        other => return Parsed::Usage(format!("unknown subcommand: {other}")),
    };

    let mut dry_run = false;
    let mut output = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Parsed::Help,
            "--dry-run" if is_sync => dry_run = true,
            "--output" => match iter.next() {
                Some(v) => output = Some(v.clone()),
                None => return Parsed::Usage("--output requires a value".into()),
            },
            a if a.starts_with("--output=") => {
                output = Some(a.trim_start_matches("--output=").to_string());
            }
            other => return Parsed::Usage(format!("unexpected argument: {other}")),
        }
    }

    let command = if is_sync {
        Command::Sync { dry_run }
    } else {
        Command::Verify
    };
    Parsed::Run(Invocation { command, output })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bare_is_usage_error() {
        assert!(matches!(parse(&[]), Parsed::Usage(_)));
    }

    #[test]
    fn help_flag() {
        assert_eq!(parse(&args(&["--help"])), Parsed::Help);
        assert_eq!(parse(&args(&["-h"])), Parsed::Help);
    }

    #[test]
    fn sync_and_flags() {
        assert_eq!(
            parse(&args(&["sync"])),
            Parsed::Run(Invocation {
                command: Command::Sync { dry_run: false },
                output: None
            })
        );
        assert_eq!(
            parse(&args(&["sync", "--dry-run", "--output", "json"])),
            Parsed::Run(Invocation {
                command: Command::Sync { dry_run: true },
                output: Some("json".into())
            })
        );
        assert_eq!(
            parse(&args(&["sync", "--output=quiet"])),
            Parsed::Run(Invocation {
                command: Command::Sync { dry_run: false },
                output: Some("quiet".into())
            })
        );
    }

    #[test]
    fn verify_takes_no_dry_run() {
        assert_eq!(
            parse(&args(&["verify"])),
            Parsed::Run(Invocation {
                command: Command::Verify,
                output: None
            })
        );
        assert!(matches!(
            parse(&args(&["verify", "--dry-run"])),
            Parsed::Usage(_)
        ));
    }

    #[test]
    fn unknown_subcommand_and_arg() {
        assert!(matches!(parse(&args(&["nope"])), Parsed::Usage(_)));
        assert!(matches!(
            parse(&args(&["sync", "--what"])),
            Parsed::Usage(_)
        ));
    }
}
