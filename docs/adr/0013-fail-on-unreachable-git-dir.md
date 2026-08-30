---
status: accepted
date: 2026-08-30
decision-makers: [Team]
---

# Fail when `.git` names a git dir that cannot be read

## Context and Problem Statement

[0012](0012-detect-branch-in-container-from-git-head.md) made the container self-sufficient: it reads the branch from the mounted repo's `.git/HEAD`. That decision noted, as a neutral consequence, that a worktree or submodule pointer file (`gitdir: …`) is followed.

Following it is not enough. A linked worktree's `.git` names a git dir *outside the working tree*, so a container that mounts only the working tree cannot read it. Detection then returned "no branch", which is the same answer as "there is no repo here", and the caller treated it as the default branch. `sync` printed `on main — no-op` and exited 0 while sitting on a feature branch.

That is the worst shape a failure can take. Nothing is rewritten, nothing is reported, and the exit code says success. A wrapper cannot detect it, CI cannot gate on it, and the user learns about it later, when a build resolves against the wrong artifacts. Worktrees are ordinary practice, and the container is the primary way to run this tool, so the two meet often.

## Decision Drivers

* A failure to detect must never be indistinguishable from success.
* The default-branch fallback must survive where absence is real: no `.git`, or a detached HEAD.
* No image growth; [0003](0003-rust-single-static-binary.md) and [0006](0006-distroless-multi-arch-image.md) hold.
* The user cannot see inside the container, so the diagnostic has to name the fix.

## Considered Options

* **A. Keep the fallback and document the mount.** No code change. The failure stays silent for everyone who has not read that paragraph, which is everyone hitting it for the first time.
* **B. Warn and continue.** Visible in a terminal, lost in a CI log, and still exit 0. It leaves the caller unable to gate.
* **C. Distinguish "unreachable" from "absent" and fail on the former.**
* **D. Resolve the branch without the git dir.** There is nothing to resolve it from: a linked worktree's working tree holds only the pointer file. Adding `git` to the image does not help either, because git follows the same pointer and fails the same way.

## Decision Outcome

Chosen option: **C**. `git::head_branch` returns `Result<String, NoBranch>` with three distinct reasons: `NotACheckout`, `Detached`, and `UnreachableGitDir(path)`. Only the last is fatal; the first two keep the default-branch no-op, which is correct there.

The diagnostic carries the path from the pointer file and both escapes: mount that path at the same location, or pass `CURRENT_BRANCH`. It stays on one line so CI reporters can render it as a single annotation.

This confines the change to the one case where evidence exists that detection failed. A `.git` that is present, is a pointer, and names an unreadable target is positive evidence, not an absence.

### Consequences

* Good, because the container stops turning a broken mount into an empty, successful-looking sync.
* Good, because the error names the two fixes, so a user who cannot inspect the container still knows what to do.
* Good, because the fallback still holds wherever the branch is genuinely absent.
* Neutral, because a stale pointer left by a deleted worktree now errors instead of no-oping. It is the same class of broken checkout, and the message fits.
* Neutral, because callers that mount only a linked worktree's working tree now fail where they used to exit 0. That exit code was the defect.
