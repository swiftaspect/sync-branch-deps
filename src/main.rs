//! `sbd` — the sync-branch-deps CLI. `sbd sync` resolves and pins branch
//! dependencies; `sbd verify` is the PR gate. Run from a consumer repo's root.
//! This binary is a thin shell around the library.

use std::env;
use std::path::Path;
use std::process::Command as ProcCommand;

use sync_branch_deps::cli::{self, Command, Parsed};
use sync_branch_deps::git;
use sync_branch_deps::reporters::{self, Reporter};

fn default_branch() -> String {
    env::var("DEFAULT_BRANCH").unwrap_or_else(|_| "main".to_string())
}

/// Current branch, in order of authority: `$CURRENT_BRANCH` (explicit override),
/// then the `git` binary (authoritative, ref-backend-agnostic — used wherever
/// git is on `PATH`), then `.git/HEAD` read directly (the fallback for a minimal
/// container with no git), then the default branch.
fn current_branch(root: &Path) -> String {
    if let Ok(b) = env::var("CURRENT_BRANCH") {
        let b = b.trim().to_string();
        if !b.is_empty() {
            return b;
        }
    }
    if let Some(b) = git_binary_branch() {
        return b;
    }
    if let Some(b) = git::head_branch(root) {
        return b;
    }
    default_branch()
}

/// `git rev-parse --abbrev-ref HEAD`, or `None` if git isn't on `PATH`, the call
/// fails, or HEAD is detached (`abbrev-ref` reports `HEAD`).
fn git_binary_branch() -> Option<String> {
    let out = ProcCommand::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!branch.is_empty() && branch != "HEAD").then_some(branch)
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
            &current_branch(&root),
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
