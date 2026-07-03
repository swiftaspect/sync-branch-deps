//! Rewriters pin a resolved coordinate wherever it is *referenced* in the repo.
//! They are keyed by coordinate *kind*, not by resolver — so an `oci` image can
//! be pinned in compose files, Kubernetes manifests, a `Containerfile` `FROM`,
//! etc. by independent rewriters, and adding one of those is a new file under
//! `rewriters/` plus a line in [`all`]. A rewriter for a file type the repo
//! doesn't have is simply a no-op.

use std::path::Path;

use anyhow::Result;

pub mod compose;
pub mod package_json;

pub trait Rewriter {
    /// The coordinate *kind* this rewriter pins (matches [`crate::resolvers::Resolver::kind`]).
    fn kind(&self) -> &'static str;

    /// Pin references to `target` to `slug` in this repo's files of this kind.
    /// Returns whether anything changed. A repo without the relevant files → `Ok(false)`.
    fn rewrite(&self, root: &Path, target: &str, slug: &str) -> Result<bool>;

    /// Find any branch / pre-release pin of `target` in this repo's files of
    /// this kind (empty = clean). Powers the `verify` gate.
    fn find_branch_pin(&self, root: &Path, target: &str) -> Result<Vec<Pin>>;
}

/// A branch / pre-release reference found by `verify`, carried with its location
/// so reporters can annotate it (e.g. a GitHub Actions `file`/`line` annotation).
#[derive(Debug, PartialEq, Eq)]
pub struct Pin {
    pub file: String,
    pub line: Option<usize>,
    pub reference: String,
}

/// Every rewriter the binary knows about.
pub fn all() -> Vec<Box<dyn Rewriter>> {
    vec![
        Box::new(package_json::PackageJson),
        Box::new(compose::Compose),
    ]
}

/// The rewriters that pin a given coordinate kind.
pub fn for_kind(kind: &str) -> Vec<Box<dyn Rewriter>> {
    all().into_iter().filter(|r| r.kind() == kind).collect()
}
