//! Pins npm coordinates in `package.json`: sets the dependency's value to the
//! branch dist-tag (the slug). Key order and 2-space indentation are preserved.

use std::path::Path;

use anyhow::{Context, Result};

use crate::rewriters::{Pin, Rewriter};

pub struct PackageJson;

impl Rewriter for PackageJson {
    fn kind(&self) -> &'static str {
        "npm"
    }

    fn rewrite(&self, root: &Path, target: &str, slug: &str) -> Result<bool> {
        let path = root.join("package.json");
        if !path.exists() {
            return Ok(false);
        }
        match rewrite(&std::fs::read_to_string(&path)?, target, slug)? {
            Some(new) => {
                std::fs::write(&path, new)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn find_branch_pin(&self, root: &Path, target: &str) -> Result<Vec<Pin>> {
        let path = root.join("package.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let doc: serde_json::Value =
            serde_json::from_str(&content).context("parsing package.json")?;
        let mut pins = Vec::new();
        for field in ["dependencies", "devDependencies", "peerDependencies"] {
            if let Some(value) = doc
                .get(field)
                .and_then(|d| d.get(target))
                .and_then(|v| v.as_str())
            {
                if is_branch_ref(value) {
                    pins.push(Pin {
                        file: "package.json".into(),
                        line: line_of(&content, target),
                        reference: format!("{target} = \"{value}\" (in {field})"),
                    });
                }
            }
        }
        Ok(pins)
    }
}

/// Whether a dependency value is an in-flight branch reference rather than a
/// released semver range. Released values are ranges (`^1.2.3`, `~1.2`, `1.2.3`,
/// `*`, …); a bare dist-tag/slug (`feat-x`, `latest`) or a pre-release
/// (`1.2.0-feat.7`) is a branch reference.
fn is_branch_ref(value: &str) -> bool {
    let first = value.chars().next().unwrap_or(' ');
    let is_range = first.is_ascii_digit() || matches!(first, '^' | '~' | '>' | '<' | '=' | '*');
    let is_prerelease = value
        .as_bytes()
        .windows(2)
        .any(|w| w[0] == b'-' && (w[1] as char).is_ascii_alphabetic());
    !is_range || is_prerelease
}

/// The 1-based line of the first occurrence of `"key"` in the raw JSON.
fn line_of(content: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\"");
    content
        .lines()
        .position(|l| l.contains(&needle))
        .map(|i| i + 1)
}

/// Rewrite `pkg`'s value to `dist_tag` across dependencies/devDependencies/
/// peerDependencies. Returns the new content if anything changed.
fn rewrite(content: &str, pkg: &str, dist_tag: &str) -> Result<Option<String>> {
    let mut doc: serde_json::Value =
        serde_json::from_str(content).context("parsing package.json")?;
    let mut changed = false;
    for field in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(deps) = doc.get_mut(field).and_then(|v| v.as_object_mut()) {
            if let Some(cur) = deps.get(pkg) {
                if cur != dist_tag {
                    deps.insert(
                        pkg.to_string(),
                        serde_json::Value::String(dist_tag.to_string()),
                    );
                    changed = true;
                }
            }
        }
    }
    if !changed {
        return Ok(None);
    }
    let mut out = serde_json::to_string_pretty(&doc).context("serializing package.json")?;
    out.push('\n');
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_matching_dep_and_preserves_order() {
        let input = "{\n  \"name\": \"consumer\",\n  \"dependencies\": {\n    \"@acme/lib\": \"^1.2.0\",\n    \"zzz\": \"^1.0.0\"\n  }\n}\n";
        let out = rewrite(input, "@acme/lib", "feat-x").unwrap().unwrap();
        assert!(out.contains("\"@acme/lib\": \"feat-x\""));
        assert!(out.find("\"name\"").unwrap() < out.find("\"dependencies\"").unwrap());
        assert!(out.find("@acme/lib").unwrap() < out.find("zzz").unwrap());
    }

    #[test]
    fn noop_when_already_pinned_or_absent() {
        let pinned = "{\n  \"dependencies\": { \"@acme/lib\": \"feat-x\" }\n}\n";
        assert!(rewrite(pinned, "@acme/lib", "feat-x").unwrap().is_none());
        let absent = "{\n  \"dependencies\": { \"other\": \"^1.0.0\" }\n}\n";
        assert!(rewrite(absent, "@acme/lib", "feat-x").unwrap().is_none());
    }

    #[test]
    fn branch_refs_vs_released_ranges() {
        // released ranges — not branch refs
        for v in ["^1.2.3", "~1.2", "1.2.3", "1.2", ">=1.0.0", "*"] {
            assert!(!is_branch_ref(v), "{v} should be released");
        }
        // dist-tags / slugs / pre-releases — branch refs
        for v in ["feat-x", "latest", "feat-new-types", "1.2.0-feat.7"] {
            assert!(is_branch_ref(v), "{v} should be a branch ref");
        }
    }

    #[test]
    fn line_lookup() {
        let content = "{\n  \"dependencies\": {\n    \"@acme/lib\": \"feat-x\"\n  }\n}\n";
        assert_eq!(line_of(content, "@acme/lib"), Some(3));
        assert_eq!(line_of(content, "missing"), None);
    }
}
