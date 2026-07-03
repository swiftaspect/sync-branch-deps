//! Container images on any OCI-Distribution registry (ghcr.io, Docker Hub,
//! Quay, GitLab, a private registry…). Existence is a manifest `HEAD` that
//! follows the registry's `WWW-Authenticate: Bearer` challenge for a token.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};

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
            let token = bearer_token(&resp).with_context(|| format!("authenticating to {host}"))?;
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

fn bearer_token(resp: &ureq::Response) -> Result<String> {
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
    let body = ureq::get(&url)
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
}
