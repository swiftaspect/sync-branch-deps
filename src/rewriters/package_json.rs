//! Pins npm coordinates in `package.json`: sets the dependency's value to the
//! branch dist-tag (the slug). Key order and 2-space indentation are preserved.

use std::path::Path;

use anyhow::{Context, Result};

use crate::rewriters::Rewriter;

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
}
