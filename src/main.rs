//! `sbd` — the sync-branch-deps CLI. Run it from a consumer repo's root; it
//! reads `.sync-branch-deps.yaml` and pins the repo's manifests to any sibling
//! pre-release artifacts published for the current branch. This binary is a thin
//! shell around the library, which holds the resolve/rewrite logic.

use std::env;
use std::process::Command;

use anyhow::{Context, Result};
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
    if let Ok(out) = Command::new("git")
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

/// Output format from `--output <fmt>` / `--output=<fmt>`, else `$SBD_OUTPUT`,
/// else auto-detected.
fn output_choice() -> Option<String> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(v) = arg.strip_prefix("--output=") {
            return Some(v.to_string());
        }
        if arg == "--output" {
            return args.next();
        }
    }
    env::var("SBD_OUTPUT").ok()
}

fn main() {
    let reporter = reporters::select(output_choice().as_deref());
    if let Err(e) = run(reporter.as_ref()) {
        reporter.error(&format!("{e:#}"));
        std::process::exit(1);
    }
}

fn run(reporter: &dyn Reporter) -> Result<()> {
    let root = env::current_dir().context("resolving working directory")?;
    sync_branch_deps::run(&root, &current_branch(), &default_branch(), reporter)
}
