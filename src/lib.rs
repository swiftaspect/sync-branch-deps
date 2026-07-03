//! sync-branch-deps: resolve cross-repo feature-branch dependencies and pin
//! this repo's manifests to the matching sibling pre-release artifacts.
//!
//! The flow is two decoupled steps per configured coordinate:
//! [`resolvers`] answer *does a branch artifact exist?*, and [`rewriters`] pin
//! the reference *wherever it lives*, matched by coordinate kind. The tool only
//! resolves and rewrites — it never invokes a package manager. Progress is
//! rendered by a [`reporters::Reporter`] chosen for the run's environment.

pub mod config;
pub mod reporters;
pub mod resolvers;
pub mod rewriters;
pub mod slug;

use std::path::Path;

use anyhow::{Context, Result};

use crate::reporters::Reporter;

/// Resolve and pin branch dependencies for the repo rooted at `root`, given the
/// current `branch`. Progress is reported through `reporter`. On the default
/// branch, a detached `HEAD`, or with no config, it is a no-op.
pub fn run(root: &Path, branch: &str, default_branch: &str, reporter: &dyn Reporter) -> Result<()> {
    if slug::is_default_branch(branch, default_branch) {
        let shown = if branch.is_empty() { "unknown" } else { branch };
        reporter.info(&format!(
            "on {shown} — no-op (branch deps resolve off feature branches only)"
        ));
        return Ok(());
    }
    let slug = slug::sanitize(branch);
    reporter.info(&format!("branch={branch} sanitized={slug}"));

    let config_path = root.join(config::CONFIG_FILE);
    if !config_path.exists() {
        reporter.info(&format!("{} absent — no-op", config::CONFIG_FILE));
        return Ok(());
    }
    let config = config::Config::parse(&std::fs::read_to_string(&config_path)?)?;

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
