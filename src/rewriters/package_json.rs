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
/// released, mergeable state. Branch references are the bare dist-tag/slug this
/// tool pins (`feat-x`, `latest`) or a semver pre-release (`1.2.0-feat.7`,
/// `1.2.0-7`). Released semver ranges (`^1.2.3`, `~1.2`, `1.2.3`, `*`) are not,
/// and neither are explicit protocol/path specifiers (`workspace:*`,
/// `file:../lib`, `link:../lib`, `git+https://…`, `github:org/repo`, `npm:`
/// aliases, URL tarballs) — those are intentional and are never a dist-tag pin
/// sbd writes. (A hand-authored git ref that happens to point at a branch, e.g.
/// `github:org/repo#feat`, is out of scope: sbd manages dist-tag pins, not git
/// refs, and flagging every `workspace:`/`file:` dep would be worse.)
fn is_branch_ref(value: &str) -> bool {
    // Protocol / path / URL specifiers carry a ':' or '/'; a dist-tag/slug or a
    // semver pre-release never does. Treat those as intentional, not a pin.
    if value.contains(':') || value.contains('/') {
        return false;
    }
    let first = value.chars().next().unwrap_or(' ');
    let is_range = first.is_ascii_digit() || matches!(first, '^' | '~' | '>' | '<' | '=' | '*');
    // A '-' immediately after a digit marks a semver pre-release (`1.2.0-feat`,
    // `1.2.0-7`); this excludes hyphen ranges (`1.2.3 - 2.0.0`), whose '-' is
    // space-separated from the version.
    let is_prerelease = value
        .as_bytes()
        .windows(2)
        .any(|w| w[0].is_ascii_digit() && w[1] == b'-');
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
    fn protocol_specifiers_are_not_branch_refs() {
        // Explicit workspace / path / git / alias / URL deps are intentional,
        // not branch pins — the gate must not flag them.
        for v in [
            "workspace:*",
            "workspace:^1.2.0",
            "file:../lib",
            "link:../lib",
            "git+https://github.com/org/repo.git",
            "github:org/repo#v1.2.3",
            "npm:@acme/lib@1.2.3",
            "https://example.com/lib-1.2.3.tgz",
        ] {
            assert!(!is_branch_ref(v), "{v} should not be a branch ref");
        }
    }

    #[test]
    fn numeric_prereleases_are_branch_refs() {
        for v in ["1.2.0-7", "1.2.0-20260703", "1.2.0-0"] {
            assert!(is_branch_ref(v), "{v} should be a branch ref");
        }
        // A hyphen range (spaces around '-') is a released range, not a pin.
        assert!(!is_branch_ref("1.2.3 - 2.0.0"));
    }

    #[test]
    fn line_lookup() {
        let content = "{\n  \"dependencies\": {\n    \"@acme/lib\": \"feat-x\"\n  }\n}\n";
        assert_eq!(line_of(content, "@acme/lib"), Some(3));
        assert_eq!(line_of(content, "missing"), None);
    }
}
