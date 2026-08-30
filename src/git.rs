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

use std::fmt;
use std::path::{Path, PathBuf};

/// Why a checkout yielded no branch name.
#[derive(Debug, PartialEq, Eq)]
pub enum NoBranch {
    /// No readable `.git` at the root: not a checkout.
    NotACheckout,
    /// `.git` is a pointer file naming a git dir that cannot be read from here,
    /// carried as the path it names. A linked worktree or submodule stores HEAD
    /// there, so the branch is unknown rather than absent.
    UnreachableGitDir(PathBuf),
    /// HEAD names no branch: a detached checkout, or a ref-backend sentinel.
    Detached,
}

impl fmt::Display for NoBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotACheckout => write!(f, "no readable `.git` in the working directory"),
            Self::UnreachableGitDir(path) => write!(
                f,
                "cannot read the git dir that `.git` points to: {}. \
                 This checkout is a linked worktree or a submodule, and that path is not visible from here. \
                 In a container it lies outside the mount. \
                 Mount it at the same path, or pass the branch as CURRENT_BRANCH.",
                path.display()
            ),
            Self::Detached => write!(f, "HEAD names no branch"),
        }
    }
}

/// The checked-out branch from `<root>/.git/HEAD`.
pub fn head_branch(root: &Path) -> Result<String, NoBranch> {
    let (git_dir, from_pointer) = git_dir(root)?;
    let head = std::fs::read_to_string(git_dir.join("HEAD")).map_err(|_| {
        if from_pointer {
            NoBranch::UnreachableGitDir(git_dir)
        } else {
            NoBranch::NotACheckout
        }
    })?;
    branch_from_head(&head).ok_or(NoBranch::Detached)
}

/// Resolve `<root>/.git` to a git dir, and whether a pointer file named it.
/// Normally `.git` is a directory; in a linked worktree or a submodule it is a
/// file holding `gitdir: <path>` (absolute, or relative to the repo root)
/// pointing at the real git dir.
fn git_dir(root: &Path) -> Result<(PathBuf, bool), NoBranch> {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return Ok((dot_git, false));
    }
    let pointer = std::fs::read_to_string(&dot_git).map_err(|_| NoBranch::NotACheckout)?;
    let named = pointer
        .trim()
        .strip_prefix("gitdir:")
        .map(str::trim)
        .ok_or(NoBranch::NotACheckout)?;
    let path = Path::new(named);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    Ok((path, true))
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
        assert_eq!(head_branch(dir.path()).as_deref(), Ok("release/1.2"));
    }

    #[test]
    fn follows_a_gitdir_pointer_file() {
        // A worktree/submodule: `.git` is a file pointing at the real git dir.
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real-git");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("HEAD"), "ref: refs/heads/wt\n").unwrap();
        std::fs::write(dir.path().join(".git"), "gitdir: real-git\n").unwrap();
        assert_eq!(head_branch(dir.path()).as_deref(), Ok("wt"));
    }

    #[test]
    fn missing_git_is_not_a_checkout() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(head_branch(dir.path()), Err(NoBranch::NotACheckout));
    }

    #[test]
    fn detached_head_in_a_git_dir_reports_detached() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(
            dir.path().join(".git/HEAD"),
            "9d3a1f2c4b5e6a7089badc0ffee1234567890abcd\n",
        )
        .unwrap();
        assert_eq!(head_branch(dir.path()), Err(NoBranch::Detached));
    }

    #[test]
    fn unreachable_gitdir_pointer_is_distinct_from_no_branch() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("elsewhere/worktrees/wt");
        std::fs::write(
            dir.path().join(".git"),
            format!("gitdir: {}\n", absent.display()),
        )
        .unwrap();
        assert_eq!(
            head_branch(dir.path()),
            Err(NoBranch::UnreachableGitDir(absent))
        );
    }

    /// The diagnostic has to carry the path and both escapes, or the caller is
    /// left guessing at a failure they cannot see inside the container.
    #[test]
    fn unreachable_gitdir_message_names_the_path_and_both_escapes() {
        let msg = NoBranch::UnreachableGitDir(PathBuf::from("/repo/.git/worktrees/wt")).to_string();
        assert!(msg.contains("/repo/.git/worktrees/wt"));
        assert!(msg.contains("CURRENT_BRANCH"));
        assert!(msg.contains("Mount it at the same path"));
        assert!(
            !msg.contains('\n'),
            "single line, for CI annotations: {msg}"
        );
    }
}
