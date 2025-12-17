use serde::Serialize;

use crate::engine::{Severity, Violation};

use super::Reporter;

pub struct JsonReporter {
    pretty: bool,
}

impl JsonReporter {
    pub fn new() -> Self {
        Self { pretty: true }
    }

    pub fn compact(mut self) -> Self {
        self.pretty = false;
        self
    }
}

impl Default for JsonReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, serde::Deserialize)]
struct JsonOutput {
    violations: Vec<JsonViolation>,
    summary: JsonSummary,
}

#[derive(Serialize, serde::Deserialize)]
struct JsonViolation {
    path: String,
    rule: String,
    message: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<String>,
}

#[derive(Serialize, serde::Deserialize)]
struct JsonSummary {
    total: usize,
    errors: usize,
    warnings: usize,
}

impl Reporter for JsonReporter {
    fn report(&self, violations: &[Violation]) -> String {
        let mut sorted_violations: Vec<_> = violations.iter().collect();
        sorted_violations.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });

        let json_violations: Vec<JsonViolation> = sorted_violations
            .iter()
            .map(|v| JsonViolation {
                path: v.path.to_string(),
                rule: v.rule_id.to_string(),
                message: v.message.to_string(),
                severity: v.severity.to_string(),
                fix: v.fix_suggestion.as_ref().map(|s| s.to_string()),
            })
            .collect();

        let error_count = violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count();
        let warning_count = violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count();

        let output = JsonOutput {
            violations: json_violations,
            summary: JsonSummary {
                total: violations.len(),
                errors: error_count,
                warnings: warning_count,
            },
        };

        if self.pretty {
            serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
        } else {
            serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_str::CompactString;

    #[test]
    fn test_json_reporter_empty() {
        let reporter = JsonReporter::new();
        let output = reporter.report(&[]);
        let parsed: JsonOutput = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed.violations.len(), 0);
        assert_eq!(parsed.summary.total, 0);
    }

    #[test]
    fn test_json_reporter_with_violations() {
        let reporter = JsonReporter::new();
        let violations = vec![Violation {
            path: CompactString::new("src/utils/helper.ts"),
            rule_id: CompactString::new("forbidPaths"),
            message: CompactString::new("path matches forbidden pattern"),
            severity: Severity::Error,
            fix_suggestion: None,
        }];

        let output = reporter.report(&violations);
        let parsed: JsonOutput = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed.violations.len(), 1);
        assert_eq!(parsed.summary.errors, 1);
    }

    #[test]
    fn test_json_reporter_compact() {
        let reporter = JsonReporter::new().compact();
        let violations = vec![Violation {
            path: CompactString::new("test.ts"),
            rule_id: CompactString::new("test"),
            message: CompactString::new("test"),
            severity: Severity::Error,
            fix_suggestion: None,
        }];

        let output = reporter.report(&violations);
        assert!(!output.contains('\n'));
    }
}
