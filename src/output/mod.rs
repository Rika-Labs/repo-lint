pub mod console;
pub mod json;
pub mod sarif;

pub use console::*;
pub use json::*;
pub use sarif::*;

use crate::engine::Violation;

pub trait Reporter {
    fn report(&self, violations: &[Violation]) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Console,
    Json,
    Sarif,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "console" | "text" => Ok(OutputFormat::Console),
            "json" => Ok(OutputFormat::Json),
            "sarif" => Ok(OutputFormat::Sarif),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

pub fn create_reporter(format: OutputFormat) -> Box<dyn Reporter> {
    match format {
        OutputFormat::Console => Box::new(ConsoleReporter::new()),
        OutputFormat::Json => Box::new(JsonReporter::new()),
        OutputFormat::Sarif => Box::new(SarifReporter::new()),
    }
}
