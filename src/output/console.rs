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
            violation.path.as_str().white().bold().to_string()
        } else {
            violation.path.to_string()
        };

        let mut output = format!(
            "{}{}: {}\n  --> {}\n",
            severity_str, rule_id, violation.message, path_str
        );

        if !violation.attempts.is_empty() {
            output.push_str("\n  Tried to match:\n");
            for attempt in &violation.attempts {
                let status = if attempt.matched {
                    if self.use_color {
                        "✓".green().to_string()
                    } else {
                        "✓".to_string()
                    }
                } else if self.use_color {
                    "✗".red().to_string()
                } else {
                    "✗".to_string()
                };
                let reason = attempt
                    .reason
                    .as_ref()
                    .map(|r| format!(" ({})", r))
                    .unwrap_or_default();
                output.push_str(&format!("    {} {}{}\n", status, attempt.pattern, reason));
            }
        }

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
        sorted_violations
            .sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.rule_id.cmp(&b.rule_id)));

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
                if warning_count == 1 {
                    "warning"
                } else {
                    "warnings"
                }
            )
            .bold()
            .to_string()
        } else {
            format!(
                "{} {} and {} {} found.",
                error_count,
                if error_count == 1 { "error" } else { "errors" },
                warning_count,
                if warning_count == 1 {
                    "warning"
                } else {
                    "warnings"
                }
            )
        };

        output.push_str(&summary);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_str::CompactString;

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
                path: CompactString::new("src/utils/helper.ts"),
                rule_id: CompactString::new("forbidPaths"),
                message: CompactString::new("path matches forbidden pattern"),
                severity: Severity::Error,
                fix_suggestion: None,
                attempts: Vec::new(),
            },
            Violation {
                path: CompactString::new("src/temp.ts"),
                rule_id: CompactString::new("forbidNames"),
                message: CompactString::new("forbidden name"),
                severity: Severity::Warning,
                fix_suggestion: Some(CompactString::new("rename to something else")),
                attempts: Vec::new(),
            },
        ];

        let output = reporter.report(&violations);
        assert!(output.contains("error"));
        assert!(output.contains("warning"));
        assert!(output.contains("forbidPaths"));
        assert!(output.contains("forbidNames"));
    }
}
