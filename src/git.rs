//! Read the checked-out branch straight from `.git/HEAD` — no `git` binary.
//!
//! This is a **fallback** for environments with no `git` on `PATH`: `sbd` runs
//! in a minimal container (distroless has no `git`) against the consumer repo
//! mounted at the working directory, and the branch it should resolve for is
//! already sitting in that repo's `.git/HEAD` — so it reads it itself rather
//! than making every caller detect and inject `CURRENT_BRANCH`.
//!
//! Where `git` *is* available (any host) the caller prefers the binary, which is
//! authoritative and agnostic to the ref-storage backend. Text-parsing `.git/HEAD`
//! assumes the classic "files" layout; a repo on the newer `reftable` backend
//! writes a `refs/heads/.invalid` sentinel here, which we reject (leading-dot
//! refnames are invalid per `git check-ref-format`) so it degrades to the
//! default-branch no-op instead of a bogus pin.

use std::path::{Path, PathBuf};

/// The checked-out branch from `<root>/.git/HEAD`, or `None` when HEAD is
/// detached (holds a raw commit id) or `.git` is absent/unreadable.
pub fn head_branch(root: &Path) -> Option<String> {
    let head = std::fs::read_to_string(git_dir(root)?.join("HEAD")).ok()?;
    branch_from_head(&head)
}

/// Resolve `<root>/.git`. Normally a directory; in a linked worktree or a
/// submodule it is a file holding `gitdir: <path>` (absolute, or relative to the
/// repo root) pointing at the real git dir.
fn git_dir(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let pointer = std::fs::read_to_string(&dot_git).ok()?;
    let path = Path::new(pointer.trim().strip_prefix("gitdir:")?.trim());
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    })
}

/// Parse `.git/HEAD`: `ref: refs/heads/<branch>` → `Some(branch)`; a raw commit
/// id (detached HEAD), the `reftable` `.invalid` sentinel, or anything else →
/// `None`. A leading-dot branch is rejected because `git check-ref-format`
/// forbids it, so it can only be a sentinel, never a real branch.
fn branch_from_head(head: &str) -> Option<String> {
    let reference = head.trim().strip_prefix("ref:")?.trim();
    let branch = reference.strip_prefix("refs/heads/")?;
    (!branch.is_empty() && !branch.starts_with('.')).then(|| branch.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_symbolic_ref_including_slashes() {
        assert_eq!(
            branch_from_head("ref: refs/heads/feat/new-types\n").as_deref(),
            Some("feat/new-types")
        );
        // No space after the colon is still valid per the format.
        assert_eq!(
            branch_from_head("ref:refs/heads/main").as_deref(),
            Some("main")
        );
    }

    #[test]
    fn detached_head_or_junk_is_none() {
        // A raw commit id: detached HEAD, no branch.
        assert_eq!(
            branch_from_head("9d3a1f2c4b5e6a7089badc0ffee1234567890abcd"),
            None
        );
        assert_eq!(branch_from_head("ref: refs/tags/v1.0.0"), None);
        assert_eq!(branch_from_head("ref: refs/heads/"), None);
        assert_eq!(branch_from_head(""), None);
        // The reftable backend writes this sentinel into `.git/HEAD`.
        assert_eq!(branch_from_head("ref: refs/heads/.invalid"), None);
    }

    #[test]
    fn reads_branch_from_a_git_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(
            dir.path().join(".git/HEAD"),
            "ref: refs/heads/release/1.2\n",
        )
        .unwrap();
        assert_eq!(head_branch(dir.path()).as_deref(), Some("release/1.2"));
    }

    #[test]
    fn follows_a_gitdir_pointer_file() {
        // A worktree/submodule: `.git` is a file pointing at the real git dir.
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real-git");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("HEAD"), "ref: refs/heads/wt\n").unwrap();
        std::fs::write(dir.path().join(".git"), "gitdir: real-git\n").unwrap();
        assert_eq!(head_branch(dir.path()).as_deref(), Some("wt"));
    }

    #[test]
    fn missing_git_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(head_branch(dir.path()), None);
    }
}
