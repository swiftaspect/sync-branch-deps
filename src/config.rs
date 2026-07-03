//! The `.sync-branch-deps.yaml` schema. It is a flat map from an ecosystem key
//! (`npm`, `oci`, `pypi`, …) for the resolver to the list of that ecosystems's
//! rewriter that sbd will manage.
//! Nothing is auto-discovered — a repo declares exactly what it wants
//! resolved and rewritten.
//! Unknown keys are surfaced by the orchestrator, not rejected here,
//! so a newer config can be read by an older binary without a hard failure.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Deserialize;

pub const CONFIG_FILE: &str = ".sync-branch-deps.yaml";

#[derive(Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct Config {
    /// ecosystem key → targets (package names or image prefixes).
    pub entries: BTreeMap<String, Vec<String>>,
}

impl Config {
    pub fn parse(yaml: &str) -> Result<Config> {
        serde_yaml::from_str(yaml).context("parsing .sync-branch-deps.yaml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ecosystem_keyed_lists() {
        let cfg = Config::parse(
            "npm:\n  - \"@org/lib\"\noci:\n  - ghcr.io/org/svc\n  - quay.io/org/other\n",
        )
        .unwrap();
        assert_eq!(cfg.entries["npm"], ["@org/lib"]);
        assert_eq!(cfg.entries["oci"], ["ghcr.io/org/svc", "quay.io/org/other"]);
    }

    #[test]
    fn empty_config_is_empty() {
        assert!(Config::parse("{}").unwrap().entries.is_empty());
    }
}
