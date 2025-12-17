use colored::Colorize;

use crate::engine::{Severity, Violation};

use super::Reporter;

pub struct ConsoleReporter {
    use_color: bool,
}

impl ConsoleReporter {
    pub fn new() -> Self {
        Self { use_color: true }
    }

    pub fn no_color(mut self) -> Self {
        self.use_color = false;
        self
    }

    fn format_violation(&self, violation: &Violation) -> String {
        let severity_str = match violation.severity {
            Severity::Error => {
                if self.use_color {
                    "error".red().bold().to_string()
                } else {
                    "error".to_string()
                }
            }
            Severity::Warning => {
                if self.use_color {
                    "warning".yellow().bold().to_string()
                } else {
                    "warning".to_string()
                }
            }
        };

        let rule_id = if self.use_color {
            format!("[{}]", violation.rule_id).cyan().to_string()
        } else {
            format!("[{}]", violation.rule_id)
        };

        let path_str = if self.use_color {
            violation.path.display().to_string().white().bold().to_string()
        } else {
            violation.path.display().to_string()
        };

        let mut output = format!(
            "{}{}: {}\n  --> {}\n",
            severity_str, rule_id, violation.message, path_str
        );

        if let Some(ref fix) = violation.fix_suggestion {
            let fix_str = if self.use_color {
                format!("  = fix: {}", fix).green().to_string()
            } else {
                format!("  = fix: {}", fix)
            };
            output.push_str(&fix_str);
            output.push('\n');
        }

        output
    }
}

impl Default for ConsoleReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Reporter for ConsoleReporter {
    fn report(&self, violations: &[Violation]) -> String {
        if violations.is_empty() {
            return if self.use_color {
                "No violations found.".green().to_string()
            } else {
                "No violations found.".to_string()
            };
        }

        let mut output = String::new();
        let mut sorted_violations: Vec<_> = violations.iter().collect();
        sorted_violations.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });

        for violation in sorted_violations {
            output.push_str(&self.format_violation(violation));
            output.push('\n');
        }

        let error_count = violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count();
        let warning_count = violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count();

        let summary = if self.use_color {
            format!(
                "{} {} and {} {} found.",
                error_count,
                if error_count == 1 { "error" } else { "errors" },
                warning_count,
                if warning_count == 1 { "warning" } else { "warnings" }
            )
            .bold()
            .to_string()
        } else {
            format!(
                "{} {} and {} {} found.",
                error_count,
                if error_count == 1 { "error" } else { "errors" },
                warning_count,
                if warning_count == 1 { "warning" } else { "warnings" }
            )
        };

        output.push_str(&summary);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_console_reporter_empty() {
        let reporter = ConsoleReporter::new().no_color();
        let output = reporter.report(&[]);
        assert_eq!(output, "No violations found.");
    }

    #[test]
    fn test_console_reporter_with_violations() {
        let reporter = ConsoleReporter::new().no_color();
        let violations = vec![
            Violation {
                path: PathBuf::from("src/utils/helper.ts"),
                rule_id: "forbidPaths".to_string(),
                message: "path matches forbidden pattern".to_string(),
                severity: Severity::Error,
                fix_suggestion: None,
            },
            Violation {
                path: PathBuf::from("src/temp.ts"),
                rule_id: "forbidNames".to_string(),
                message: "forbidden name".to_string(),
                severity: Severity::Warning,
                fix_suggestion: Some("rename to something else".to_string()),
            },
        ];

        let output = reporter.report(&violations);
        assert!(output.contains("error"));
        assert!(output.contains("warning"));
        assert!(output.contains("forbidPaths"));
        assert!(output.contains("forbidNames"));
    }
}
