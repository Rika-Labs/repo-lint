use serde::Serialize;

use crate::engine::{Severity, Violation};

use super::Reporter;

pub struct SarifReporter;

impl SarifReporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SarifReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct SarifOutput {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: String,
    version: String,
    #[serde(rename = "informationUri")]
    information_uri: String,
    rules: Vec<SarifRule>,
}

#[derive(Serialize)]
struct SarifRule {
    id: String,
    name: String,
    #[serde(rename = "shortDescription")]
    short_description: SarifMessage,
    #[serde(rename = "defaultConfiguration")]
    default_configuration: SarifDefaultConfiguration,
}

#[derive(Serialize)]
struct SarifDefaultConfiguration {
    level: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

impl Reporter for SarifReporter {
    fn report(&self, violations: &[Violation]) -> String {
        let mut sorted_violations: Vec<_> = violations.iter().collect();
        sorted_violations.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });

        let mut rule_ids: Vec<String> = violations
            .iter()
            .map(|v| v.rule_id.clone())
            .collect();
        rule_ids.sort();
        rule_ids.dedup();

        let rules: Vec<SarifRule> = rule_ids
            .iter()
            .map(|id| SarifRule {
                id: id.clone(),
                name: id.clone(),
                short_description: SarifMessage {
                    text: format!("repo-lint {} rule", id),
                },
                default_configuration: SarifDefaultConfiguration {
                    level: "error".to_string(),
                },
            })
            .collect();

        let results: Vec<SarifResult> = sorted_violations
            .iter()
            .map(|v| SarifResult {
                rule_id: v.rule_id.clone(),
                level: match v.severity {
                    Severity::Error => "error".to_string(),
                    Severity::Warning => "warning".to_string(),
                },
                message: SarifMessage {
                    text: v.message.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: v.path.display().to_string(),
                        },
                    },
                }],
            })
            .collect();

        let output = SarifOutput {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            version: "2.1.0".to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "repo-lint".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        information_uri: "https://github.com/rika-labs/repo-lint".to_string(),
                        rules,
                    },
                },
                results,
            }],
        };

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sarif_reporter_empty() {
        let reporter = SarifReporter::new();
        let output = reporter.report(&[]);
        assert!(output.contains("\"version\": \"2.1.0\""));
        assert!(output.contains("\"results\": []"));
    }

    #[test]
    fn test_sarif_reporter_with_violations() {
        let reporter = SarifReporter::new();
        let violations = vec![Violation {
            path: PathBuf::from("src/utils/helper.ts"),
            rule_id: "forbidPaths".to_string(),
            message: "path matches forbidden pattern".to_string(),
            severity: Severity::Error,
            fix_suggestion: None,
        }];

        let output = reporter.report(&violations);
        assert!(output.contains("forbidPaths"));
        assert!(output.contains("src/utils/helper.ts"));
        assert!(output.contains("\"level\": \"error\""));
    }
}
