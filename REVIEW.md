<!--
  === Copied from copier-template-typescript ===
  Source: template/REVIEW.md at commit
  891960849e52ec2afd7ec8bc2076a6cbbdec2dfd. This repo owns this copy; a
  change here does not need a template change.
-->

# Review policy

Every pull request gets the same review passes, in this order. The reviewer reads this file first and applies only what it says. A human merges. The reviewer never approves, never edits files, and never pushes.

## Passes

1. **Correctness.** Logic errors, unhandled failure paths, and behavior that differs from what the issue in the `Refs` line asks for. Read the issue before the diff.
2. **Security.** Secrets or credentials in the diff. Input that crosses a trust boundary without validation. A permission or authentication check that the change removes or bypasses.
3. **Conventions.** The rules a reader cannot see from the diff alone:
   - The change stays inside the issue's scope. A file move, a package relocation, or a redesign the issue does not name is a finding.
   - No file listed in `.copier-managed-files` changed.
   - No existing test was weakened to make the code pass.
   - A comment says why, never what, and never cites where a rule came from.
   - Every commit follows Conventional Commits and its body carries the rationale.
   - The pull request body starts with `Refs`, has an "Assumptions made" section, and states what was verified and what was not.

## Important and nit

**Important** blocks the merge. A finding is important when it causes a defect, a security hole, a reverted commit, a broken contract with another repository, or a scope violation.

**Nit** does not block. Naming, wording, formatting, and style are nits. Post at most five nits. When more exist, count them and name the class once.

## Excluded paths

Do not review these. Other tools own them.

- Files listed in `.copier-managed-files`. Report a change to one as an important finding under Conventions, and review nothing inside it.
- `uv.lock`, `dist/`.
- `CHANGELOG.md` and `.release-please-manifest.json`.

## Output

One comment, in this order:

1. **Important**, ranked by severity. Each finding names the file and line, states the defect in one sentence, and gives the concrete input or state that triggers it.
2. **Nits**, at most five.
3. **Not reviewed**: paths skipped and why, and any context the reviewer could not reach.

When there is nothing to report in a section, write "None". Plain, literal language. One idea per sentence. No praise.
