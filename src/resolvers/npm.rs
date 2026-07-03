//! npm packages. A branch build publishes a pre-release under a dist-tag named
//! for the branch slug; resolution asks the registry which version that dist-tag
//! points at. Registry and auth come from `.npmrc` (project then user), so this
//! works against npmjs.org, GitHub Packages, or any private npm registry.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::resolvers::Resolver;

pub struct Npm;

impl Resolver for Npm {
    fn key(&self) -> &'static str {
        "npm"
    }

    fn kind(&self) -> &'static str {
        "npm"
    }

    fn resolve(&self, root: &Path, target: &str, slug: &str) -> Result<Option<String>> {
        let npmrc = Npmrc::load(root);
        let registry = npmrc.registry_for(target);
        let token = npmrc.token_for(&registry);
        fetch_dist_tag(&registry, token.as_deref(), target, slug)
    }
}

/// A minimal `.npmrc` view: scope→registry mappings and per-registry auth
/// tokens, with `${VAR}` environment expansion (as npm itself does).
#[derive(Default)]
struct Npmrc {
    entries: BTreeMap<String, String>,
}

impl Npmrc {
    fn load(root: &Path) -> Npmrc {
        let mut entries = BTreeMap::new();
        // User config first, then project — project wins on conflict.
        if let Some(home) = std::env::var_os("HOME") {
            merge(&std::path::Path::new(&home).join(".npmrc"), &mut entries);
        }
        merge(&root.join(".npmrc"), &mut entries);
        Npmrc { entries }
    }

    #[cfg(test)]
    fn parse(text: &str) -> Npmrc {
        let mut entries = BTreeMap::new();
        merge_text(text, &mut entries);
        Npmrc { entries }
    }

    /// The registry a package resolves against: a scoped `@scope:registry`
    /// setting, else the default `registry`, else npmjs.org.
    fn registry_for(&self, name: &str) -> String {
        if let Some(scope) = scope_of(name) {
            if let Some(r) = self.entries.get(&format!("{scope}:registry")) {
                return r.clone();
            }
        }
        self.entries
            .get("registry")
            .cloned()
            .unwrap_or_else(|| "https://registry.npmjs.org".to_string())
    }

    /// The `_authToken` configured for a registry's host, if any.
    fn token_for(&self, registry: &str) -> Option<String> {
        let host = registry
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let host = host.split('/').next().unwrap_or(host);
        self.entries
            .iter()
            .find(|(k, _)| k.starts_with(&format!("//{host}")) && k.ends_with(":_authToken"))
            .map(|(_, v)| v.clone())
    }
}

fn scope_of(name: &str) -> Option<&str> {
    if name.starts_with('@') {
        name.split('/').next()
    } else {
        None
    }
}

fn merge(path: &Path, out: &mut BTreeMap<String, String>) {
    if let Ok(text) = std::fs::read_to_string(path) {
        merge_text(&text, out);
    }
}

fn merge_text(text: &str, out: &mut BTreeMap<String, String>) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.insert(k.trim().to_string(), expand_env(v.trim()));
        }
    }
}

/// Expand `${VAR}` references from the environment (unset → empty, as npm does).
fn expand_env(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find('}') {
            Some(end) => {
                let var = &rest[start + 2..start + 2 + end];
                out.push_str(&std::env::var(var).unwrap_or_default());
                rest = &rest[start + 2 + end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Fetch the packument and return the version behind dist-tag `slug`, if present.
fn fetch_dist_tag(
    registry: &str,
    token: Option<&str>,
    name: &str,
    slug: &str,
) -> Result<Option<String>> {
    // Scoped names encode the `/` in the path segment (`@scope%2Fname`).
    let url = format!(
        "{}/{}",
        registry.trim_end_matches('/'),
        name.replace('/', "%2F")
    );
    let mut req = ureq::get(&url).set("Accept", "application/json");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    match req.call() {
        Ok(resp) => Ok(dist_tag(&resp.into_string()?, slug)),
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(ureq::Error::Status(code, _)) => bail!("npm registry {url} → HTTP {code}"),
        Err(e) => Err(e).with_context(|| format!("querying npm registry for {name}")),
    }
}

/// Extract `dist-tags.<slug>` from a packument body.
fn dist_tag(packument: &str, slug: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(packument).ok()?;
    json.get("dist-tags")?
        .get(slug)?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_extraction() {
        assert_eq!(scope_of("@acme/lib"), Some("@acme"));
        assert_eq!(scope_of("plain-pkg"), None);
    }

    #[test]
    fn npmrc_resolves_scoped_registry_and_token() {
        let rc = Npmrc::parse(
            "@acme:registry=https://npm.example.com\n\
             //npm.example.com/:_authToken=secret-123\n\
             registry=https://registry.npmjs.org\n",
        );
        assert_eq!(rc.registry_for("@acme/lib"), "https://npm.example.com");
        assert_eq!(rc.registry_for("plain-pkg"), "https://registry.npmjs.org");
        assert_eq!(
            rc.token_for("https://npm.example.com"),
            Some("secret-123".to_string())
        );
        assert_eq!(rc.token_for("https://registry.npmjs.org"), None);
    }

    #[test]
    fn npmrc_skips_comments_and_blanks() {
        let rc = Npmrc::parse("# a comment\n\n; another\nregistry=https://r.example.com\n");
        assert_eq!(rc.registry_for("x"), "https://r.example.com");
    }

    #[test]
    fn env_expansion() {
        std::env::set_var("SBD_TEST_TOKEN", "xyz");
        assert_eq!(expand_env("Bearer ${SBD_TEST_TOKEN}"), "Bearer xyz");
        assert_eq!(expand_env("no-vars-here"), "no-vars-here");
        std::env::remove_var("SBD_TEST_TOKEN");
    }

    #[test]
    fn dist_tag_lookup() {
        let body =
            r#"{"name":"@acme/lib","dist-tags":{"latest":"1.2.0","feat-x":"1.2.0-feat-x.7"}}"#;
        assert_eq!(dist_tag(body, "feat-x"), Some("1.2.0-feat-x.7".to_string()));
        assert_eq!(dist_tag(body, "nope"), None);
    }
}
