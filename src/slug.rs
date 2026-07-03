//! Branch-name → registry-safe slug. This is the shared convention every
//! ecosystem resolves against, and it must match how the publish side names
//! branch artifacts (`sed 's/[^[:alnum:]]/-/g'` is the base):
//! every non-alphanumeric character becomes `-`, with no lowercasing and no
//! collapsing of runs.

/// Convert a branch name to its slug.
pub fn sanitize(branch: &str) -> String {
    branch
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Whether a branch should be treated as a no-op (a release context, not a
/// feature branch): the default branch, a detached `HEAD`, or empty.
pub fn is_default_branch(branch: &str, default_branch: &str) -> bool {
    branch.is_empty() || branch == "HEAD" || branch == default_branch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_matches_publish_rule() {
        assert_eq!(sanitize("feat/new-types"), "feat-new-types");
        assert_eq!(sanitize("feat/New_Types.2"), "feat-New-Types-2");
        assert_eq!(sanitize("main"), "main");
        assert_eq!(sanitize("release/1.2"), "release-1-2");
    }

    #[test]
    fn default_branch_is_noop() {
        assert!(is_default_branch("main", "main"));
        assert!(is_default_branch("HEAD", "main"));
        assert!(is_default_branch("", "main"));
        assert!(!is_default_branch("feat/x", "main"));
    }
}
