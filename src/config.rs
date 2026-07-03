//! The `.sync-branch-deps.yaml` schema. It is a flat map from an ecosystem key
//! (`npm`, `oci`, `pypi`, …) for the resolver to the list of that ecosystems's
//! rewriter that sbd will manage.
//! Nothing is auto-discovered — a repo declares exactly what it wants
//! resolved and rewritten.
//! Unknown keys are surfaced by the orchestrator, not rejected here, and any
//! value that isn't an ecosystem's list of targets is set aside (and surfaced
//! as a warning) rather than failing the parse — so a newer config (new keys,
//! or added scalar schema keys like `version: 2`) is read by an older binary
//! without a hard failure.

use std::collections::BTreeMap;

use anyhow::{Context, Result};

pub const CONFIG_FILE: &str = ".sync-branch-deps.yaml";

#[derive(Debug, Default)]
pub struct Config {
    /// ecosystem key → targets (package names or image prefixes).
    pub entries: BTreeMap<String, Vec<String>>,
    /// Keys whose value wasn't a list of targets (a scalar from a newer schema,
    /// or a typo). Kept so the orchestrator can warn instead of the parse
    /// hard-failing — see decisions/0009.
    pub ignored: Vec<String>,
}

impl Config {
    pub fn parse(yaml: &str) -> Result<Config> {
        let raw: BTreeMap<String, serde_yaml::Value> =
            serde_yaml::from_str(yaml).context("parsing .sync-branch-deps.yaml")?;
        // Forward-compat: an ecosystem maps to a list of targets. A value of any
        // other shape (a scalar, a map, or an added scalar schema key like
        // `version: 2`) can't be acted on, so it's recorded for the orchestrator
        // to warn about rather than hard-failing the run — see decisions/0009.
        // `raw` is sorted, so `entries` and `ignored` stay sorted too.
        let mut config = Config::default();
        for (key, value) in raw {
            match serde_yaml::from_value::<Vec<String>>(value) {
                Ok(targets) => {
                    config.entries.insert(key, targets);
                }
                Err(_) => config.ignored.push(key),
            }
        }
        Ok(config)
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

    #[test]
    fn ignores_non_list_values_for_forward_compat() {
        // A newer config may hand an existing key a non-list value or add a
        // scalar-valued schema key (e.g. `version: 2`); an older binary must
        // read it without a hard failure, keeping the entries it understands
        // and recording the rest (sorted) for the orchestrator to warn about.
        let cfg = Config::parse("npm:\n  - \"@org/lib\"\nversion: 2\noci: not-a-list\n").unwrap();
        assert_eq!(cfg.entries["npm"], ["@org/lib"]);
        assert!(!cfg.entries.contains_key("version"));
        assert!(!cfg.entries.contains_key("oci"));
        assert_eq!(cfg.ignored, ["oci", "version"]);
    }
}
