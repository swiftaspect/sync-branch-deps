//! Resolvers answer one question per artifact kind: **does a branch artifact
//! exist for this coordinate?** They are pure registry existence checks — no
//! files are touched here (that's the rewriters' job). Adding an artifact kind
//! (PyPI, crates.io, Go, Maven, …) is a new file under `resolvers/` plus one
//! line in [`all`].

use std::path::Path;

use anyhow::Result;

pub mod npm;
pub mod oci;

pub trait Resolver {
    /// The `.sync-branch-deps.yaml` key this resolver reads (e.g. `"npm"`).
    fn key(&self) -> &'static str;

    /// Whether this resolver handles a config key. Defaults to an exact match on
    /// [`Resolver::key`]; override to accept aliases.
    fn handles(&self, key: &str) -> bool {
        key == self.key()
    }

    /// The coordinate *kind* this resolver produces (e.g. `"npm"`, `"oci"`).
    /// Rewriters register against the same kind, so resolution and pinning stay
    /// decoupled: an `oci` coordinate can be pinned by compose *and* k8s
    /// rewriters without the resolver knowing about either.
    fn kind(&self) -> &'static str;

    /// Ask the registry whether `target` has a branch artifact for `slug`.
    /// `Ok(Some(id))` = found (resolved version/tag, for logging); `Ok(None)` =
    /// a miss; `Err` = a lookup failure (network/auth), which is distinct.
    fn resolve(&self, root: &Path, target: &str, slug: &str) -> Result<Option<String>>;
}

/// Every resolver the binary knows about.
pub fn all() -> Vec<Box<dyn Resolver>> {
    vec![Box::new(npm::Npm), Box::new(oci::Oci)]
}

/// The resolver that handles a given config key, if any.
pub fn for_key(key: &str) -> Option<Box<dyn Resolver>> {
    all().into_iter().find(|r| r.handles(key))
}
