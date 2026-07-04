//! Container images on any OCI-Distribution registry (ghcr.io, Docker Hub,
//! Quay, GitLab, a private registry…). Existence is a manifest `HEAD` that
//! follows the registry's `WWW-Authenticate: Bearer` challenge for a token.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::resolvers::Resolver;

const ACCEPT_MANIFESTS: &str = "application/vnd.oci.image.manifest.v1+json, \
     application/vnd.docker.distribution.manifest.v2+json, \
     application/vnd.oci.image.index.v1+json, \
     application/vnd.docker.distribution.manifest.list.v2+json";

pub struct Oci;

impl Resolver for Oci {
    fn key(&self) -> &'static str {
        "oci"
    }

    // `images` is the historical/intuitive key; accept it as an alias for `oci`.
    fn handles(&self, key: &str) -> bool {
        key == "oci" || key == "images"
    }

    fn kind(&self) -> &'static str {
        "oci"
    }

    fn resolve(&self, _root: &Path, target: &str, slug: &str) -> Result<Option<String>> {
        if tag_exists(target, slug)? {
            Ok(Some(slug.to_string()))
        } else {
            Ok(None)
        }
    }
}

/// Split an image prefix into `(api_host, repository)` for the OCI Distribution
/// API. `ghcr.io/org/img` → (`ghcr.io`, `org/img`). A prefix with no host segment
/// defaults to Docker Hub, whose API host is `registry-1.docker.io` and whose
/// single-segment repos live under `library/`.
pub fn split_image_prefix(prefix: &str) -> (String, String) {
    let (host, repo) = match prefix.split_once('/') {
        Some((first, rest))
            if first.contains('.') || first.contains(':') || first == "localhost" =>
        {
            (first.to_string(), rest.to_string())
        }
        _ => ("docker.io".to_string(), prefix.to_string()),
    };
    if host == "docker.io" || host == "registry-1.docker.io" {
        let repo = if repo.contains('/') {
            repo
        } else {
            format!("library/{repo}")
        };
        ("registry-1.docker.io".to_string(), repo)
    } else {
        (host, repo)
    }
}

/// Parse a `WWW-Authenticate: Bearer realm="…",service="…",scope="…"` challenge.
pub fn parse_bearer_challenge(header: &str) -> BTreeMap<String, String> {
    let rest = header
        .trim()
        .strip_prefix("Bearer ")
        .unwrap_or(header.trim());
    let mut params = BTreeMap::new();
    for part in rest.split(',') {
        if let Some((k, v)) = part.split_once('=') {
            params.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    params
}

fn tag_exists(image_prefix: &str, tag: &str) -> Result<bool> {
    let (host, repo) = split_image_prefix(image_prefix);
    let url = format!("https://{host}/v2/{repo}/manifests/{tag}");
    match head(&url, None) {
        Ok(_) => Ok(true),
        Err(ureq::Error::Status(404, _)) => Ok(false),
        Err(ureq::Error::Status(401, resp)) => {
            let token =
                bearer_token(&resp, &host).with_context(|| format!("authenticating to {host}"))?;
            match head(&url, Some(&token)) {
                Ok(_) => Ok(true),
                Err(ureq::Error::Status(404, _)) => Ok(false),
                Err(ureq::Error::Status(code, _)) => {
                    bail!("{image_prefix}:{tag} → HTTP {code} after auth")
                }
                Err(e) => Err(e).with_context(|| format!("checking {image_prefix}:{tag}")),
            }
        }
        Err(ureq::Error::Status(code, _)) => bail!("{image_prefix}:{tag} → HTTP {code}"),
        Err(e) => Err(e).with_context(|| format!("checking {image_prefix}:{tag}")),
    }
}

// ureq::Error is a large enum; returning it by value keeps the status-code match
// above flat and readable, and this runs a handful of times per invocation.
// See decisions/0001-unboxed-ureq-error-in-registry-client.md.
#[allow(clippy::result_large_err)]
fn head(url: &str, token: Option<&str>) -> std::result::Result<ureq::Response, ureq::Error> {
    let mut req = ureq::head(url).set("Accept", ACCEPT_MANIFESTS);
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    req.call()
}

fn bearer_token(resp: &ureq::Response, host: &str) -> Result<String> {
    let challenge = resp
        .header("WWW-Authenticate")
        .context("registry returned 401 without a WWW-Authenticate challenge")?;
    let params = parse_bearer_challenge(challenge);
    let realm = params
        .get("realm")
        .context("auth challenge missing realm")?;
    let mut url = realm.clone();
    let mut sep = '?';
    for key in ["service", "scope"] {
        if let Some(val) = params.get(key) {
            url.push(sep);
            url.push_str(key);
            url.push('=');
            url.push_str(val);
            sep = '&';
        }
    }
    // Present registry credentials to the authorization server so it issues a
    // *scoped* token. A branch-dep workflow resolves mostly private images, for
    // which an anonymous token carries no pull scope — the manifest HEAD would
    // then 401 again. With no credential configured we stay anonymous, which
    // still resolves public images. See credential source order in `credential_for`.
    let mut req = ureq::get(&url);
    if let Some(cred) = credential_for(host) {
        req = req.set("Authorization", &format!("Basic {cred}"));
    }
    let body = req
        .call()
        .context("requesting registry token")?
        .into_string()?;
    let json: serde_json::Value =
        serde_json::from_str(&body).context("parsing registry token response")?;
    json.get("token")
        .or_else(|| json.get("access_token"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .context("token response missing 'token'/'access_token'")
}

/// Resolve a per-host HTTP Basic credential from the standard, runtime-agnostic
/// OCI credential sources, in precedence order:
///
/// 1. `$REGISTRY_AUTH_FILE` — an explicit path to an auth document;
/// 2. `$DOCKER_AUTH_CONFIG` — an auth document passed *inline* (the common CI
///    convention for handing registry creds to a job without a file on disk);
/// 3. the default auth-file locations a registry login writes to
///    (`$XDG_RUNTIME_DIR/containers/auth.json`, `$DOCKER_CONFIG/config.json`,
///    `~/.config/containers/auth.json`, `~/.docker/config.json`).
///
/// These env vars and paths are cross-tool conventions (their names are fixed by
/// history, not by any one engine). Every source stores the credential the same
/// way — a base64(`user:pass`) blob under `auths.<host>.auth`, which is exactly
/// an HTTP Basic value — so nothing is decoded or re-encoded. Absence leaves the
/// caller anonymous (fine for public images). See `decisions/0011`.
fn credential_for(host: &str) -> Option<String> {
    auth_documents()
        .into_iter()
        .find_map(|doc| credential_from_auth_json(&doc, host))
}

/// The auth documents to search, highest precedence first: an explicitly pointed
/// file, then the inline `$DOCKER_AUTH_CONFIG`, then the default auth files.
fn auth_documents() -> Vec<String> {
    let mut docs = Vec::new();
    if let Some(path) = non_empty(std::env::var("REGISTRY_AUTH_FILE").ok()) {
        if let Ok(body) = std::fs::read_to_string(path) {
            docs.push(body);
        }
    }
    if let Some(inline) = non_empty(std::env::var("DOCKER_AUTH_CONFIG").ok()) {
        docs.push(inline);
    }
    for path in default_auth_files() {
        if let Ok(body) = std::fs::read_to_string(path) {
            docs.push(body);
        }
    }
    docs
}

/// The default auth-file locations, in the conventional lookup order (runtime
/// state dir first, then the config-home fallbacks).
fn default_auth_files() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(rt) = non_empty(std::env::var("XDG_RUNTIME_DIR").ok()) {
        paths.push(PathBuf::from(rt).join("containers/auth.json"));
    }
    if let Some(dc) = non_empty(std::env::var("DOCKER_CONFIG").ok()) {
        paths.push(PathBuf::from(dc).join("config.json"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        paths.push(home.join(".config/containers/auth.json"));
        paths.push(home.join(".docker/config.json"));
    }
    paths
}

/// Extract `auths.<host>.auth` from an OCI auth-config document. Pure over the
/// document text so it can be tested without touching the filesystem or env.
fn credential_from_auth_json(body: &str, host: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    let auths = json.get("auths")?.as_object()?;
    host_keys(host).into_iter().find_map(|key| {
        auths
            .get(&key)?
            .get("auth")
            .and_then(Value::as_str)
            .filter(|a| !a.is_empty())
            .map(str::to_string)
    })
}

/// Keys an auth file might store a host under: the bare API host and its
/// `https://` form, plus Docker Hub's legacy `https://index.docker.io/v1/`.
fn host_keys(host: &str) -> Vec<String> {
    let mut keys = vec![host.to_string(), format!("https://{host}")];
    if host == "registry-1.docker.io" || host == "docker.io" {
        keys.push("index.docker.io".to_string());
        keys.push("https://index.docker.io/v1/".to_string());
    }
    keys
}

fn non_empty(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_oci_and_images_aliases() {
        assert!(Oci.handles("oci"));
        assert!(Oci.handles("images"));
        assert!(!Oci.handles("npm"));
    }

    #[test]
    fn split_is_registry_agnostic() {
        assert_eq!(
            split_image_prefix("ghcr.io/acme/service"),
            ("ghcr.io".into(), "acme/service".into())
        );
        assert_eq!(
            split_image_prefix("quay.io/acme/other"),
            ("quay.io".into(), "acme/other".into())
        );
        assert_eq!(
            split_image_prefix("localhost:5000/img"),
            ("localhost:5000".into(), "img".into())
        );
        assert_eq!(
            split_image_prefix("nginx"),
            ("registry-1.docker.io".into(), "library/nginx".into())
        );
        assert_eq!(
            split_image_prefix("bitnami/postgresql"),
            ("registry-1.docker.io".into(), "bitnami/postgresql".into())
        );
    }

    #[test]
    fn parses_bearer_challenge_params() {
        let h = r#"Bearer realm="https://ghcr.io/token",service="ghcr.io",scope="repository:acme/service:pull""#;
        let p = parse_bearer_challenge(h);
        assert_eq!(p["realm"], "https://ghcr.io/token");
        assert_eq!(p["service"], "ghcr.io");
        assert_eq!(p["scope"], "repository:acme/service:pull");
    }

    #[test]
    fn reads_credential_for_bare_and_url_host_keys() {
        let doc = r#"{"auths":{"ghcr.io":{"auth":"Z2g6dG9r"}}}"#;
        assert_eq!(
            credential_from_auth_json(doc, "ghcr.io").as_deref(),
            Some("Z2g6dG9r")
        );

        let url = r#"{"auths":{"https://quay.io":{"auth":"cXVheTp0"}}}"#;
        assert_eq!(
            credential_from_auth_json(url, "quay.io").as_deref(),
            Some("cXVheTp0")
        );
    }

    #[test]
    fn docker_hub_matches_legacy_index_key() {
        // The API host is `registry-1.docker.io`, but a login stores Docker Hub
        // under its legacy `https://index.docker.io/v1/` key.
        let doc = r#"{"auths":{"https://index.docker.io/v1/":{"auth":"ZGg6dA=="}}}"#;
        assert_eq!(
            credential_from_auth_json(doc, "registry-1.docker.io").as_deref(),
            Some("ZGg6dA==")
        );
    }

    #[test]
    fn missing_host_empty_blob_or_malformed_is_none() {
        let doc = r#"{"auths":{"ghcr.io":{"auth":"Z2g6dG9r"}}}"#;
        assert_eq!(credential_from_auth_json(doc, "quay.io"), None);
        // An entry with no usable `auth` (e.g. a credsStore-only record) is a miss.
        assert_eq!(
            credential_from_auth_json(r#"{"auths":{"ghcr.io":{"auth":""}}}"#, "ghcr.io"),
            None
        );
        assert_eq!(
            credential_from_auth_json(r#"{"auths":{"ghcr.io":{}}}"#, "ghcr.io"),
            None
        );
        assert_eq!(credential_from_auth_json("not json", "ghcr.io"), None);
        assert_eq!(credential_from_auth_json("{}", "ghcr.io"), None);
    }
}
