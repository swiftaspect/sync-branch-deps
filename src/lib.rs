//! sync-branch-deps: resolve cross-repo feature-branch dependencies and pin
//! this repo's manifests to the matching pre-release artifacts.
//!
//! Two operations, both scoped to the *manifest* half of the model:
//! [`sync`] resolves branch artifacts and pins them (the write side), and
//! [`verify`] asserts no branch pin remains — the PR gate (the read side).
//! [`resolvers`] answer *does a branch artifact exist?* and [`rewriters`] pin or
//! detect the reference, matched by coordinate kind. The tool only resolves and
//! rewrites — it never invokes a package manager.

pub mod cli;
pub mod config;
pub mod reporters;
pub mod resolvers;
pub mod rewriters;
pub mod slug;

use std::path::Path;

use anyhow::{Context, Result};

use crate::reporters::{Level, Location, Reporter};

/// Resolve and pin branch dependencies for the repo at `root`, given the current
/// `branch`. On the default branch, a detached `HEAD`, or with no config, it is
/// a no-op. With `dry_run`, it reports what it *would* pin without writing.
pub fn sync(
    root: &Path,
    branch: &str,
    default_branch: &str,
    dry_run: bool,
    reporter: &dyn Reporter,
) -> Result<()> {
    if slug::is_default_branch(branch, default_branch) {
        let shown = if branch.is_empty() { "unknown" } else { branch };
        reporter.info(&format!(
            "on {shown} — no-op (branch deps resolve off feature branches only)"
        ));
        return Ok(());
    }
    let slug = slug::sanitize(branch);
    reporter.info(&format!("branch={branch} sanitized={slug}"));

    let Some(config) = load_config(root, reporter)? else {
        return Ok(());
    };

    for (key, targets) in &config.entries {
        let Some(resolver) = resolvers::for_key(key) else {
            reporter.warn(&format!("{key}: no resolver for this key — skipping"));
            continue;
        };
        let rewriters = rewriters::for_kind(resolver.kind());
        for target in targets {
            match resolver
                .resolve(root, target, &slug)
                .with_context(|| format!("resolving {target}"))?
            {
                Some(id) if dry_run => {
                    reporter.notice(&format!("{target}: would pin to '{slug}' (resolved {id})"));
                }
                Some(id) => {
                    let mut changed = false;
                    for rw in &rewriters {
                        changed |= rw.rewrite(root, target, &slug)?;
                    }
                    if changed {
                        reporter.notice(&format!("{target}: pinned to '{slug}' (resolved {id})"));
                    } else {
                        reporter.info(&format!("{target}: already at '{slug}'"));
                    }
                }
                None => reporter.info(&format!("{target}: no '{slug}' — skipping")),
            }
        }
    }
    Ok(())
}

/// The PR gate: scan the manifests for any branch / pre-release pin of a declared
/// coordinate, reporting each as a located diagnostic. Returns `true` if clean.
pub fn verify(root: &Path, reporter: &dyn Reporter) -> Result<bool> {
    let Some(config) = load_config(root, reporter)? else {
        return Ok(true);
    };

    let mut clean = true;
    for (key, targets) in &config.entries {
        let Some(resolver) = resolvers::for_key(key) else {
            reporter.warn(&format!("{key}: no resolver for this key — skipping"));
            continue;
        };
        let rewriters = rewriters::for_kind(resolver.kind());
        for target in targets {
            for rw in &rewriters {
                for pin in rw.find_branch_pin(root, target)? {
                    clean = false;
                    reporter.report_located(
                        Level::Error,
                        &Location {
                            file: &pin.file,
                            line: pin.line,
                        },
                        &format!(
                            "branch/pre-release pin must be reverted before merge: {}",
                            pin.reference
                        ),
                    );
                }
            }
        }
    }
    if clean {
        reporter.info("no branch pins found");
    }
    Ok(clean)
}

/// Load `.sync-branch-deps.yaml`; `Ok(None)` (with a note) when absent.
fn load_config(root: &Path, reporter: &dyn Reporter) -> Result<Option<config::Config>> {
    let path = root.join(config::CONFIG_FILE);
    if !path.exists() {
        reporter.info(&format!("{} absent — nothing to do", config::CONFIG_FILE));
        return Ok(None);
    }
    let config = config::Config::parse(&std::fs::read_to_string(&path)?)?;
    // A value that isn't a list of targets can't be acted on; surface each so a
    // forward-compat scalar or a typo is visible rather than silently ignored.
    for key in &config.ignored {
        reporter.warn(&format!("{key}: not a list of targets — ignoring"));
    }
    Ok(Some(config))
}
