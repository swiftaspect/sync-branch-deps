//! End-to-end tests of the public `sync` and `verify` entry points, through the
//! crate's public API only. These cover the no-network paths (default branch,
//! missing config, unknown key) and the `verify` gate (clean vs. branch-pinned),
//! so they are hermetic. Resolver network behavior is covered by unit tests.

use std::fs;
use std::path::Path;

use sync_branch_deps::{reporters, sync, verify};

fn quiet() -> Box<dyn reporters::Reporter> {
    reporters::select(Some("quiet"))
}

const CONFIG: &str = "npm:\n  - \"@acme/lib\"\noci:\n  - ghcr.io/acme/svc\n";

/// On the default branch, sync is a no-op even with a config present.
#[test]
fn sync_no_op_on_default_branch() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".sync-branch-deps.yaml"), CONFIG).unwrap();
    sync(dir.path(), "main", "main", false, quiet().as_ref()).unwrap();
}

/// With no config, a feature branch is a no-op.
#[test]
fn sync_no_op_without_config() {
    let dir = tempfile::tempdir().unwrap();
    sync(dir.path(), "feat/x", "main", false, quiet().as_ref()).unwrap();
    assert!(!dir.path().join(".sync-branch-deps.yaml").exists());
}

/// A config key no resolver handles is warned and skipped — no network, no
/// file changes.
#[test]
fn sync_unknown_ecosystem_key_touches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".sync-branch-deps.yaml"),
        "totally-unknown:\n  - whatever\n",
    )
    .unwrap();
    let pkg = "{\n  \"dependencies\": {}\n}\n";
    fs::write(dir.path().join("package.json"), pkg).unwrap();

    sync(dir.path(), "feat/x", "main", false, quiet().as_ref()).unwrap();

    assert_eq!(read(dir.path(), "package.json"), pkg);
}

/// `verify` passes when declared coordinates are pinned to released versions.
#[test]
fn verify_passes_when_clean() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".sync-branch-deps.yaml"), CONFIG).unwrap();
    fs::write(
        dir.path().join("package.json"),
        "{\n  \"dependencies\": { \"@acme/lib\": \"^1.2.0\" }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("compose.yaml"),
        "services:\n  svc:\n    image: ghcr.io/acme/svc:1.2.3\n",
    )
    .unwrap();

    assert!(verify(dir.path(), quiet().as_ref()).unwrap());
}

/// `verify` fails when a branch pin is present in either manifest.
#[test]
fn verify_fails_on_branch_pins() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".sync-branch-deps.yaml"), CONFIG).unwrap();
    fs::write(
        dir.path().join("package.json"),
        "{\n  \"dependencies\": { \"@acme/lib\": \"feat-x\" }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("compose.yaml"),
        "services:\n  svc:\n    image: ghcr.io/acme/svc:feat-x\n",
    )
    .unwrap();

    assert!(!verify(dir.path(), quiet().as_ref()).unwrap());
}

/// `verify` must catch a branch pin even when the compose image value is quoted
/// — quoting is valid YAML and previously slipped through the gate.
#[test]
fn verify_fails_on_quoted_branch_pin() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".sync-branch-deps.yaml"), CONFIG).unwrap();
    fs::write(
        dir.path().join("compose.yaml"),
        "services:\n  svc:\n    image: \"ghcr.io/acme/svc:feat-x\"\n",
    )
    .unwrap();

    assert!(!verify(dir.path(), quiet().as_ref()).unwrap());
}

/// A `base_image:` field and a commented-out image line must not be mistaken
/// for a service image, so an otherwise-released repo still passes the gate.
#[test]
fn verify_ignores_non_image_keys_and_comments() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".sync-branch-deps.yaml"),
        "oci:\n  - ghcr.io/acme/svc\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("compose.yaml"),
        "services:\n  svc:\n    image: ghcr.io/acme/svc:1.2.3\n    base_image: ghcr.io/acme/svc:feat-x\n    # image: ghcr.io/acme/svc:feat-x\n",
    )
    .unwrap();

    assert!(verify(dir.path(), quiet().as_ref()).unwrap());
}

fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(name)).unwrap()
}
