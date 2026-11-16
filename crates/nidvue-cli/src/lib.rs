// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Dot-path access to the configuration.
//!
//! This is the crate's library half: `nidvue-cli` is the binary, and the tests
//! that keep every configuration key addressable live here.
//!
//! Every value in `config.toml` is reachable from the command line without a
//! flag per key: `nidvue-cli config get cache.max_size_bytes`. The mapping is
//! the serialised form of `AppConfig`, so a key that exists in the file exists
//! here and a key that does not is an error naming the closest match.

pub mod config_dot {
    use nidvue_core::config::AppConfig;
    use nidvue_core::error::{NidvueError, Result};

    /// A configuration tree as JSON, which is what the dot paths walk.
    pub fn tree(config: &AppConfig) -> Result<serde_json::Value> {
        serde_json::to_value(config)
            .map_err(|e| NidvueError::Config(format!("configuration is not serialisable: {e}")))
    }

    /// Read one leaf by dot path.
    pub fn get(config: &AppConfig, key: &str) -> Result<serde_json::Value> {
        let tree = tree(config)?;
        walk(&tree, key)
            .cloned()
            .ok_or_else(|| unknown_key(&tree, key))
    }

    /// Set one leaf by dot path, returning a configuration that still validates.
    ///
    /// The value is parsed against the type already there, so `true` sets a bool and
    /// `10` a number without the caller naming the type.
    pub fn set(config: &AppConfig, key: &str, raw: &str) -> Result<AppConfig> {
        let mut tree = tree(config)?;
        let existing = walk(&tree, key).ok_or_else(|| unknown_key(&tree, key))?;
        let value = coerce(existing, raw)?;
        let suggestion = unknown_key(&tree, key);
        let slot = walk_mut(&mut tree, key).ok_or(suggestion)?;
        *slot = value;
        let updated: AppConfig =
            serde_json::from_value(tree).map_err(|e| NidvueError::Config(format!("{key}: {e}")))?;
        updated.validate()?;
        Ok(updated)
    }

    /// Every leaf, as `(dot path, value)` pairs in document order.
    pub fn list(config: &AppConfig) -> Result<Vec<(String, serde_json::Value)>> {
        let tree = tree(config)?;
        let mut out = Vec::new();
        flatten(&tree, String::new(), &mut out);
        Ok(out)
    }

    /// Every leaf path, for documentation and for the coverage test.
    pub fn keys(config: &AppConfig) -> Result<Vec<String>> {
        Ok(list(config)?.into_iter().map(|(k, _)| k).collect())
    }

    fn flatten(
        value: &serde_json::Value,
        prefix: String,
        out: &mut Vec<(String, serde_json::Value)>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    flatten(child, path, out);
                }
            }
            serde_json::Value::Array(items) if !items.is_empty() => {
                for (index, child) in items.iter().enumerate() {
                    flatten(child, format!("{prefix}.{index}"), out);
                }
            }
            other => out.push((prefix, other.clone())),
        }
    }

    fn walk<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
        let mut current = value;
        for part in key.split('.').filter(|p| !p.is_empty()) {
            current = match current {
                serde_json::Value::Object(map) => map.get(part)?,
                serde_json::Value::Array(items) => items.get(part.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    fn walk_mut<'a>(
        value: &'a mut serde_json::Value,
        key: &str,
    ) -> Option<&'a mut serde_json::Value> {
        let mut current = value;
        for part in key.split('.').filter(|p| !p.is_empty()) {
            current = match current {
                serde_json::Value::Object(map) => map.get_mut(part)?,
                serde_json::Value::Array(items) => items.get_mut(part.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    fn coerce(existing: &serde_json::Value, raw: &str) -> Result<serde_json::Value> {
        let trimmed = raw.trim();
        let bad = |what: &str| NidvueError::Config(format!("expected {what}, got {raw:?}"));
        match existing {
            serde_json::Value::Bool(_) => trimmed
                .parse::<bool>()
                .map(serde_json::Value::Bool)
                .map_err(|_| bad("true or false")),
            serde_json::Value::Number(number) => {
                if number.is_f64() {
                    trimmed
                        .parse::<f64>()
                        .map(|v| serde_json::json!(v))
                        .map_err(|_| bad("a number"))
                } else if number.is_u64() {
                    trimmed
                        .parse::<u64>()
                        .map(|v| serde_json::json!(v))
                        .map_err(|_| bad("a non-negative integer"))
                } else {
                    trimmed
                        .parse::<i64>()
                        .map(|v| serde_json::json!(v))
                        .map_err(|_| bad("an integer"))
                }
            }
            serde_json::Value::String(_) => {
                if trimmed == "null" {
                    Ok(serde_json::Value::Null)
                } else {
                    Ok(serde_json::Value::String(raw.to_string()))
                }
            }
            serde_json::Value::Null => {
                if trimmed == "null" || trimmed.is_empty() {
                    Ok(serde_json::Value::Null)
                } else {
                    Ok(serde_json::Value::String(raw.to_string()))
                }
            }
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                serde_json::from_str(trimmed)
                    .map_err(|e| NidvueError::Config(format!("{raw:?} is not valid JSON: {e}")))
            }
        }
    }

    fn unknown_key(tree: &serde_json::Value, key: &str) -> NidvueError {
        let mut all = Vec::new();
        flatten(tree, String::new(), &mut all);
        let suggestion = all
            .iter()
            .map(|(path, _)| (path, distance(path, key)))
            .filter(|(path, d)| path.ends_with(key) || *d <= 3)
            .min_by_key(|(_, d)| *d)
            .map(|(path, _)| format!("; did you mean {path}?"))
            .unwrap_or_else(|| "; run `nidvue-cli config list` for every key".to_string());
        NidvueError::Config(format!("unknown configuration key {key:?}{suggestion}"))
    }

    fn distance(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        let mut cur = vec![0usize; b.len() + 1];
        for (i, ca) in a.iter().enumerate() {
            cur[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let cost = usize::from(ca != cb);
                cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        prev[b.len()]
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn every_leaf_is_readable_by_its_own_path() {
            let config = AppConfig::default();
            let keys = keys(&config).unwrap();
            assert!(keys.len() > 40, "only {} keys found", keys.len());
            for key in &keys {
                let value = get(&config, key).unwrap_or_else(|e| panic!("{key}: {e}"));
                assert!(!value.is_object(), "{key} is not a leaf");
            }
        }

        #[test]
        fn documented_keys_are_present() {
            let config = AppConfig::default();
            let keys = keys(&config).unwrap();
            for key in [
                "state_dir",
                "cache.max_size_bytes",
                "cache.eviction_policy",
                "sync.poll_interval_ms",
                "sync.workers",
                "network.max_concurrency",
                "network.requests_per_second",
                "log.level",
                "log.otel_endpoint",
                "log.otel_interval_s",
                "log.prometheus_bind",
                "secret.backend",
                "ui.theme",
            ] {
                assert!(keys.iter().any(|k| k == key), "missing {key}");
            }
        }

        #[test]
        fn set_parses_against_the_existing_type() {
            let config = AppConfig::default();
            let updated = set(&config, "cache.max_size_bytes", "1048576").unwrap();
            assert_eq!(updated.cache.max_size_bytes, 1_048_576);

            let updated = set(&config, "sync.paused", "true").unwrap();
            assert!(updated.sync.paused);

            let updated = set(&config, "cache.high_watermark", "0.95").unwrap();
            assert!((updated.cache.high_watermark - 0.95).abs() < f64::EPSILON);

            let updated = set(&config, "log.level", "debug").unwrap();
            assert_eq!(updated.log.level, "debug");

            let updated = set(&config, "secret.backend", "keyring").unwrap();
            assert_eq!(
                updated.secret.backend,
                nidvue_core::config::SecretBackend::Keyring
            );

            let updated = set(&config, "log.prometheus_bind", "127.0.0.1:9464").unwrap();
            assert_eq!(
                updated.log.prometheus_bind.as_deref(),
                Some("127.0.0.1:9464")
            );
            let cleared = set(&updated, "log.prometheus_bind", "null").unwrap();
            assert!(cleared.log.prometheus_bind.is_none());
        }

        #[test]
        fn every_leaf_round_trips_through_set() {
            let config = AppConfig::default();
            for (key, value) in list(&config).unwrap() {
                if key.starts_with("sync.pairs.") {
                    continue;
                }
                let raw = match &value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "null".to_string(),
                    other => other.to_string(),
                };
                let updated = set(&config, &key, &raw).unwrap_or_else(|e| panic!("{key}: {e}"));
                let after = get(&updated, &key).unwrap();
                assert_eq!(after, value, "{key} did not round-trip");
            }
        }

        #[test]
        fn a_path_buf_value_is_a_string() {
            let config = AppConfig {
                state_dir: PathBuf::from("/var/lib/nidvue"),
                ..Default::default()
            };
            assert_eq!(
                get(&config, "state_dir").unwrap(),
                serde_json::json!("/var/lib/nidvue")
            );
        }

        #[test]
        fn an_unknown_key_names_a_close_one() {
            let config = AppConfig::default();
            let err = get(&config, "cache.max_size_byte").unwrap_err();
            let text = err.to_string();
            assert!(text.contains("unknown configuration key"), "{text}");
            assert!(text.contains("cache.max_size_bytes"), "{text}");

            let err = get(&config, "nonsense.key").unwrap_err();
            assert!(err.to_string().contains("config list"));
        }

        #[test]
        fn a_wrong_type_is_refused() {
            let config = AppConfig::default();
            assert!(set(&config, "cache.max_size_bytes", "lots")
                .unwrap_err()
                .to_string()
                .contains("non-negative integer"));
            assert!(set(&config, "sync.paused", "yes")
                .unwrap_err()
                .to_string()
                .contains("true or false"));
        }

        #[test]
        fn an_invalid_result_is_refused_by_validate() {
            let config = AppConfig::default();
            assert!(set(&config, "cache.max_size_bytes", "0").is_err());
            assert!(set(&config, "cache.low_watermark", "0.99").is_err());
            assert!(set(&config, "sync.workers", "0").is_err());
        }

        #[test]
        fn pairs_are_addressable_by_index() {
            let mut config = AppConfig::default();
            config.sync.pairs.push(nidvue_core::config::SyncPair {
                name: "docs".into(),
                local: PathBuf::from("/tmp/docs"),
                remote: "/Docs".into(),
                paused: false,
                include: vec![],
                exclude: vec![],
                conflict_policy: None,
            });
            assert_eq!(
                get(&config, "sync.pairs.0.name").unwrap(),
                serde_json::json!("docs")
            );
            let updated = set(&config, "sync.pairs.0.paused", "true").unwrap();
            assert!(updated.sync.pairs[0].paused);
        }
    }
}
