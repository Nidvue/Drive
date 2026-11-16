// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Metric samples and their two wire formats.
//!
//! One snapshot, rendered twice: Prometheus text exposition 0.0.4 for a scrape
//! endpoint and OTLP/HTTP JSON for an OpenTelemetry collector. Rendering lives
//! here because it is pure; the transports live in `nidvue-telemetry`, which is
//! the only other crate allowed to speak HTTP.

use serde::{Deserialize, Serialize};

/// One metric sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricSample {
    /// Prometheus-style name: `[a-z][a-z0-9_]*`.
    pub name: String,
    /// One line of help, printed as `# HELP`.
    pub help: String,
    /// The current value. Every sample here is a gauge.
    pub value: f64,
}

impl MetricSample {
    /// A sample.
    pub fn new(name: impl Into<String>, help: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            help: help.into(),
            value,
        }
    }
}

/// The snapshot both exporters publish, plus the resource attributes OTLP
/// carries and Prometheus exposes as a constant label set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Service name, e.g. `nidvue-drive`.
    pub service: String,
    /// Service version.
    pub version: String,
    /// Account name, when known.
    pub account: Option<String>,
    /// The samples.
    pub samples: Vec<MetricSample>,
}

impl MetricsSnapshot {
    /// Look one sample up by name.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.samples
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.value)
    }

    /// Render the Prometheus text exposition format (version 0.0.4).
    ///
    /// Sample names are already Prometheus-shaped, so no sanitising is needed;
    /// `[`sanitise_name`] is used by callers that build names from user data.
    pub fn render_prometheus(&self) -> String {
        let mut out = String::with_capacity(self.samples.len() * 96);
        for sample in &self.samples {
            out.push_str("# HELP ");
            out.push_str(&sample.name);
            out.push(' ');
            out.push_str(&escape_help(&sample.help));
            out.push('\n');
            out.push_str("# TYPE ");
            out.push_str(&sample.name);
            out.push_str(" gauge\n");
            out.push_str(&sample.name);
            out.push(' ');
            out.push_str(&format_value(sample.value));
            out.push('\n');
        }
        out.push_str("# HELP nidvue_build_info Build information as a constant 1.\n");
        out.push_str("# TYPE nidvue_build_info gauge\n");
        out.push_str(&format!(
            "nidvue_build_info{{version=\"{}\"}} 1\n",
            escape_label(&self.version)
        ));
        out
    }

    /// Render OTLP/HTTP JSON for `POST <endpoint>/v1/metrics`.
    ///
    /// `time_unix_nano` stamps the data points; callers pass the same clock
    /// reading for a whole export so a collector sees one batch.
    pub fn to_otlp_json(&self, time_unix_nano: u64) -> serde_json::Value {
        let mut resource_attributes = vec![
            attribute("service.name", &self.service),
            attribute("service.version", &self.version),
        ];
        if let Some(account) = &self.account {
            resource_attributes.push(attribute("enduser.name", account));
        }
        let data_points: Vec<serde_json::Value> = self
            .samples
            .iter()
            .map(|sample| {
                serde_json::json!({
                    "asDouble": sample.value,
                    "timeUnixNano": time_unix_nano.to_string(),
                })
            })
            .collect();
        let metrics: Vec<serde_json::Value> = self
            .samples
            .iter()
            .zip(data_points)
            .map(|(sample, point)| {
                serde_json::json!({
                    "name": sample.name,
                    "description": sample.help,
                    "unit": "1",
                    "gauge": { "dataPoints": [point] },
                })
            })
            .collect();
        serde_json::json!({
            "resourceMetrics": [{
                "resource": { "attributes": resource_attributes },
                "scopeMetrics": [{
                    "scope": { "name": "nidvue-drive", "version": self.version },
                    "metrics": metrics,
                }],
            }],
        })
    }
}

fn attribute(key: &str, value: &str) -> serde_json::Value {
    serde_json::json!({ "key": key, "value": { "stringValue": value } })
}

fn format_value(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            "+Inf".to_string()
        } else {
            "-Inf".to_string()
        }
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn escape_help(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}

fn escape_label(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Turn an arbitrary string into a legal Prometheus metric name.
pub fn sanitise_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (i, ch) in name.chars().enumerate() {
        let ok = ch.is_ascii_alphanumeric() || ch == '_';
        match (i, ok) {
            (0, true) if ch.is_ascii_digit() => {
                out.push('_');
                out.push(ch);
            }
            (_, true) => out.push(ch),
            (_, false) => out.push('_'),
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> MetricsSnapshot {
        MetricsSnapshot {
            service: "nidvue-drive".into(),
            version: "0.2.0".into(),
            account: Some("alice".into()),
            samples: vec![
                MetricSample::new("nidvue_cache_bytes", "Bytes stored in the cache.", 1024.0),
                MetricSample::new(
                    "nidvue_conflicts_open",
                    "Conflicts awaiting a decision.",
                    2.0,
                ),
            ],
        }
    }

    #[test]
    fn prometheus_text_is_well_formed() {
        let text = snapshot().render_prometheus();
        for line in [
            "# HELP nidvue_cache_bytes Bytes stored in the cache.",
            "# TYPE nidvue_cache_bytes gauge",
            "nidvue_cache_bytes 1024",
            "nidvue_conflicts_open 2",
            "nidvue_build_info{version=\"0.2.0\"} 1",
        ] {
            assert!(text.contains(line), "missing {line:?} in\n{text}");
        }
        assert!(text.ends_with('\n'));
        let help = text.lines().filter(|l| l.starts_with("# HELP")).count();
        assert_eq!(help, 3);
    }

    #[test]
    fn prometheus_values_keep_edge_cases() {
        let s = MetricsSnapshot {
            service: "s".into(),
            version: "1".into(),
            account: None,
            samples: vec![
                MetricSample::new("a_total", "fractional", 0.5),
                MetricSample::new("b_total", "positive infinity", f64::INFINITY),
                MetricSample::new("c_total", "not a number", f64::NAN),
            ],
        };
        let text = s.render_prometheus();
        assert!(text.contains("a_total 0.5"));
        assert!(text.contains("b_total +Inf"));
        assert!(text.contains("c_total NaN"));
    }

    #[test]
    fn otlp_json_carries_resource_scope_and_datapoints() {
        let doc = snapshot().to_otlp_json(1_700_000_000_000_000_000);
        let rm = &doc["resourceMetrics"][0];
        let attrs = rm["resource"]["attributes"].as_array().unwrap();
        assert_eq!(attrs[0]["key"], "service.name");
        assert_eq!(attrs[0]["value"]["stringValue"], "nidvue-drive");
        assert_eq!(attrs[2]["value"]["stringValue"], "alice");
        let metrics = rm["scopeMetrics"][0]["metrics"].as_array().unwrap();
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0]["name"], "nidvue_cache_bytes");
        assert_eq!(metrics[0]["gauge"]["dataPoints"][0]["asDouble"], 1024.0);
        assert_eq!(
            metrics[0]["gauge"]["dataPoints"][0]["timeUnixNano"],
            "1700000000000000000"
        );
        assert_eq!(rm["scopeMetrics"][0]["scope"]["name"], "nidvue-drive");
    }

    #[test]
    fn otlp_json_is_valid_json_round_trip() {
        let doc = snapshot().to_otlp_json(1);
        let text = serde_json::to_string(&doc).unwrap();
        let back: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn names_are_sanitised() {
        assert_eq!(sanitise_name("9lives-left.ok"), "_9lives_left_ok");
        assert_eq!(sanitise_name("ok_name"), "ok_name");
        assert_eq!(sanitise_name(""), "_");
        assert_eq!(sanitise_name("a b"), "a_b");
    }

    #[test]
    fn lookups_are_by_name() {
        assert_eq!(snapshot().get("nidvue_conflicts_open"), Some(2.0));
        assert_eq!(snapshot().get("nope"), None);
    }
}
