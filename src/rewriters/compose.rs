//! Pins `oci` coordinates in Compose files: rewrites `image: <prefix>:<tag>` to
//! the branch slug. Operates on raw text (not a YAML round-trip) so comments and
//! formatting survive. This is one of potentially several `oci` rewriters —
//! Kubernetes manifests, `Containerfile` `FROM`, Helm values, etc. would be
//! sibling files keyed to the same `oci` kind.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::rewriters::{Pin, Rewriter};

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

    fn find_branch_pin(&self, root: &Path, target: &str) -> Result<Vec<Pin>> {
        let re = image_regex(target)?;
        let mut pins = Vec::new();
        for path in compose_files(root)? {
            let file = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            for (i, line) in std::fs::read_to_string(&path)?.lines().enumerate() {
                if let Some(cap) = re.captures(line) {
                    let tag = &cap[2];
                    if is_branch_tag(tag) {
                        pins.push(Pin {
                            file: file.clone(),
                            line: Some(i + 1),
                            reference: format!("{target}:{tag}"),
                        });
                    }
                }
            }
        }
        Ok(pins)
    }
}

/// Whether an image tag is an in-flight branch reference rather than a released
/// version. Released tags are numeric-dotted with an optional `v` (`1`, `1.2`,
/// `1.2.3`, `v1.2.3`); a slug (`feat-x`) or a suffixed tag (`1.2.3-feat`,
/// `1.2.3-<sha>`) is a branch reference.
fn is_branch_tag(tag: &str) -> bool {
    let core = tag.strip_prefix('v').unwrap_or(tag);
    let released = !core.is_empty()
        && core
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
    !released
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

/// Regex for an `image:` value whose repository part is `prefix`, capturing the
/// tag. Shared by [`rewrite`] and [`Compose::find_branch_pin`] so pin and gate
/// agree on what an image reference is. It is anchored to `image:` as a
/// line-leading mapping key (after indentation and an optional `- ` list
/// marker), so it never matches inside another key like `base_image:` or a
/// `# image: …` comment, and it consumes an optional surrounding quote so
/// `image: "prefix:tag"` (valid, common YAML) is matched too. Group 1 is the
/// text a rewrite must keep (up to and including the prefix); group 2 is the
/// tag. Flow-style mappings (`{image: x}`) are intentionally not matched — block
/// style is universal for Compose. The tag class stops before a closing quote,
/// so a rewrite of a quoted value leaves the quote intact.
fn image_regex(prefix: &str) -> Result<regex::Regex> {
    let pattern = format!(
        r#"(?m)^(\s*(?:-\s+)?image:\s*["']?{}):([^\s'"]+)"#,
        regex::escape(prefix)
    );
    regex::Regex::new(&pattern).context("building compose image regex")
}

/// Rewrite `image: <prefix>:<anytag>` to `<prefix>:<tag>`. Returns the new
/// content if anything changed.
fn rewrite(content: &str, image_prefix: &str, tag: &str) -> Result<Option<String>> {
    let re = image_regex(image_prefix)?;
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
    fn rewrites_quoted_image_and_preserves_quotes() {
        for input in [
            "    image: \"ghcr.io/acme/service:1.1.4\"\n",
            "    image: 'ghcr.io/acme/service:1.1.4'\n",
        ] {
            let out = rewrite(input, "ghcr.io/acme/service", "feat-x")
                .unwrap()
                .unwrap();
            let q = input.chars().find(|c| *c == '"' || *c == '\'').unwrap();
            assert!(
                out.contains(&format!("image: {q}ghcr.io/acme/service:feat-x{q}")),
                "quote {q} should survive: {out}"
            );
        }
    }

    #[test]
    fn ignores_substring_keys_and_comments() {
        // Only the real `image:` mapping key is rewritten; a `base_image:` field
        // and a commented-out image line are left untouched.
        let input = "services:\n  a:\n    image: ghcr.io/acme/service:1.1.4\n    base_image: ghcr.io/acme/service:1.0.0\n    # image: ghcr.io/acme/service:old\n";
        let out = rewrite(input, "ghcr.io/acme/service", "feat-x")
            .unwrap()
            .unwrap();
        assert!(out.contains("    image: ghcr.io/acme/service:feat-x"));
        assert!(out.contains("base_image: ghcr.io/acme/service:1.0.0"));
        assert!(out.contains("# image: ghcr.io/acme/service:old"));
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

    #[test]
    fn branch_tags_vs_released() {
        for t in ["1", "1.2", "1.2.3", "v1.2.3"] {
            assert!(!is_branch_tag(t), "{t} should be released");
        }
        for t in ["feat-x", "1.2.3-feat", "1.2.3-20260703T0000Z", "latest"] {
            assert!(is_branch_tag(t), "{t} should be a branch tag");
        }
    }
}
