//! Pins `oci` coordinates in Compose files: rewrites `image: <prefix>:<tag>` to
//! the branch slug. Operates on raw text (not a YAML round-trip) so comments and
//! formatting survive. This is one of potentially several `oci` rewriters —
//! Kubernetes manifests, `Containerfile` `FROM`, Helm values, etc. would be
//! sibling files keyed to the same `oci` kind.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::rewriters::Rewriter;

pub struct Compose;

impl Rewriter for Compose {
    fn kind(&self) -> &'static str {
        "oci"
    }

    fn rewrite(&self, root: &Path, target: &str, slug: &str) -> Result<bool> {
        let mut changed = false;
        for path in compose_files(root)? {
            if let Some(new) = rewrite(&std::fs::read_to_string(&path)?, target, slug)? {
                std::fs::write(&path, new)?;
                changed = true;
            }
        }
        Ok(changed)
    }
}

/// Every `compose*.{yaml,yml}` file in the repo root, sorted for deterministic
/// ordering (`compose.yaml`, `compose.ci.yaml`, `compose.defaults.yaml`, …).
fn compose_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root).with_context(|| format!("reading {}", root.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if is_compose_file(name) {
                    files.push(entry.path());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Matches the `compose*.{yaml,yml}` glob.
fn is_compose_file(name: &str) -> bool {
    name.starts_with("compose") && (name.ends_with(".yaml") || name.ends_with(".yml"))
}

/// Rewrite `image: <prefix>:<anytag>` to `<prefix>:<tag>`. Returns the new
/// content if anything changed.
fn rewrite(content: &str, image_prefix: &str, tag: &str) -> Result<Option<String>> {
    let pattern = format!(r#"(image:\s*{}):[^\s'"]+"#, regex::escape(image_prefix));
    let re = regex::Regex::new(&pattern).context("building compose image regex")?;
    let replaced = re.replace_all(content, format!("${{1}}:{tag}").as_str());
    if replaced == content {
        Ok(None)
    } else {
        Ok(Some(replaced.into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_matching_prefix_only() {
        let input = "services:\n  a:\n    image: ghcr.io/acme/service:1.1.4\n  b:\n    image: ghcr.io/acme/other:1.0.5\n";
        let out = rewrite(input, "ghcr.io/acme/service", "feat-x")
            .unwrap()
            .unwrap();
        assert!(out.contains("ghcr.io/acme/service:feat-x"));
        assert!(out.contains("ghcr.io/acme/other:1.0.5"));
    }

    #[test]
    fn noop_when_prefix_absent() {
        let input = "  image: ghcr.io/acme/other:1.0.5\n";
        assert!(rewrite(input, "ghcr.io/acme/service", "feat-x")
            .unwrap()
            .is_none());
    }

    #[test]
    fn compose_glob_matches_expected_names() {
        for name in [
            "compose.yaml",
            "compose.yml",
            "compose.defaults.yaml",
            "compose.ci.yml",
        ] {
            assert!(is_compose_file(name), "{name} should match");
        }
        for name in [
            "docker-compose.yaml",
            "compose.json",
            "notes.yaml",
            "compose",
        ] {
            assert!(!is_compose_file(name), "{name} should not match");
        }
    }
}
