//! End-to-end tests of the public `run` orchestrator, exercised through the
//! crate's public API only. These cover the no-network paths — the default
//! branch, a missing config, and an unknown ecosystem key — so they are
//! hermetic. Resolver network behavior is covered by the unit tests.

use std::fs;
use std::path::Path;

use sync_branch_deps::{reporters, run};

fn quiet() -> Box<dyn reporters::Reporter> {
    reporters::select(Some("quiet"))
}

/// On the default branch, sbd is a no-op even with a config present.
#[test]
fn no_op_on_default_branch() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".sync-branch-deps.yaml"),
        "npm:\n  - \"@acme/lib\"\n",
    )
    .unwrap();
    run(dir.path(), "main", "main", quiet().as_ref()).unwrap();
}

/// With no `.sync-branch-deps.yaml`, a feature branch is a no-op.
#[test]
fn no_op_without_config() {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), "feat/x", "main", quiet().as_ref()).unwrap();
    assert!(!dir.path().join(".sync-branch-deps.yaml").exists());
}

/// A config key no resolver handles is warned and skipped — no network, no
/// file changes.
#[test]
fn unknown_ecosystem_key_touches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".sync-branch-deps.yaml"),
        "totally-unknown:\n  - whatever\n",
    )
    .unwrap();
    let pkg = "{\n  \"dependencies\": {}\n}\n";
    fs::write(dir.path().join("package.json"), pkg).unwrap();

    run(dir.path(), "feat/x", "main", quiet().as_ref()).unwrap();

    assert_eq!(read(dir.path(), "package.json"), pkg);
}

fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(name)).unwrap()
}
