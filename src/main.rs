//! `sbd` — the sync-branch-deps CLI. `sbd sync` resolves and pins branch
//! dependencies; `sbd verify` is the PR gate. Run from a consumer repo's root.
//! This binary is a thin shell around the library.

use std::env;
use std::process::Command as ProcCommand;

use sync_branch_deps::cli::{self, Command, Parsed};
use sync_branch_deps::reporters::{self, Reporter};

fn default_branch() -> String {
    env::var("DEFAULT_BRANCH").unwrap_or_else(|_| "main".to_string())
}

/// Current branch: `$CURRENT_BRANCH` (CI passes it when git isn't available in
/// the container), else `git rev-parse`, else the default branch.
fn current_branch() -> String {
    if let Ok(b) = env::var("CURRENT_BRANCH") {
        let b = b.trim().to_string();
        if !b.is_empty() {
            return b;
        }
    }
    if let Ok(out) = ProcCommand::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        if out.status.success() {
            let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !b.is_empty() {
                return b;
            }
        }
    }
    default_branch()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match cli::parse(&args) {
        Parsed::Help => println!("{}", cli::USAGE),
        Parsed::Usage(msg) => {
            eprintln!("sbd: {msg}\n\n{}", cli::USAGE);
            std::process::exit(2);
        }
        Parsed::Run(inv) => {
            let reporter = reporters::select(inv.output.as_deref());
            std::process::exit(run(inv.command, reporter.as_ref()));
        }
    }
}

fn run(command: Command, reporter: &dyn Reporter) -> i32 {
    let root = match env::current_dir() {
        Ok(r) => r,
        Err(e) => {
            reporter.error(&format!("resolving working directory: {e}"));
            return 1;
        }
    };
    let clean = match command {
        Command::Sync { dry_run } => sync_branch_deps::sync(
            &root,
            &current_branch(),
            &default_branch(),
            dry_run,
            reporter,
        )
        .map(|()| true),
        Command::Verify => sync_branch_deps::verify(&root, reporter),
    };
    match clean {
        Ok(true) => 0,
        Ok(false) => 1, // verify found branch pins
        Err(e) => {
            reporter.error(&format!("{e:#}"));
            1
        }
    }
}
