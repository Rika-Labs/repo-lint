import { Effect } from "effect";
import type { CheckResult } from "./types.js";

const RED = "\x1b[31m";
const YELLOW = "\x1b[33m";
const GRAY = "\x1b[90m";
const CYAN = "\x1b[36m";
const RESET = "\x1b[0m";

export const formatConsole = (result: CheckResult): string => {
  const lines: string[] = [];

  for (const v of result.violations) {
    const color = v.severity === "error" ? RED : YELLOW;
    const label = v.severity === "error" ? "error" : "warning";

    lines.push(`${color}${label}${RESET}[${v.rule}]: ${v.message}`);
    lines.push(`  ${GRAY}-->${RESET} ${v.path}`);

    if (v.expected !== undefined) {
      lines.push(`  ${GRAY}= expected:${RESET} ${v.expected}`);
    }

    if (v.got !== undefined) {
      lines.push(`  ${GRAY}= got:${RESET} ${v.got}`);
    }

    const firstSuggestion = v.suggestions?.[0];
    if (firstSuggestion !== undefined) {
      lines.push(`  ${GRAY}= suggestion:${RESET} ${firstSuggestion}`);
    }

    lines.push("");
  }

  const { total, errors, warnings, filesChecked, duration } = result.summary;

  if (total === 0) {
    lines.push(`${CYAN}✓${RESET} No issues found (${filesChecked} files in ${duration}ms)`);
  } else {
    const parts: string[] = [];
    if (errors > 0) parts.push(`${RED}${errors} error${errors !== 1 ? "s" : ""}${RESET}`);
    if (warnings > 0) parts.push(`${YELLOW}${warnings} warning${warnings !== 1 ? "s" : ""}${RESET}`);
    lines.push(`Found ${parts.join(" and ")} (${filesChecked} files in ${duration}ms)`);
  }

  return lines.join("\n");
};

export const formatJson = (result: CheckResult): string => JSON.stringify(result, null, 2);

export const formatSarif = (result: CheckResult): string => {
  const sarif = {
    $schema:
      "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
    version: "2.1.0",
    runs: [
      {
        tool: {
          driver: {
            name: "repo-lint",
            version: "0.5.0",
            informationUri: "https://github.com/Rika-Labs/repo-lint",
            rules: [...new Set(result.violations.map((v) => v.rule))].map((rule) => ({
              id: rule,
              shortDescription: { text: rule },
            })),
          },
        },
        results: result.violations.map((v) => ({
          ruleId: v.rule,
          level: v.severity === "error" ? "error" : "warning",
          message: { text: v.message },
          locations: [
            {
              physicalLocation: {
                artifactLocation: { uri: v.path },
                region: { startLine: v.line ?? 1, startColumn: v.column ?? 1 },
              },
            },
          ],
        })),
      },
    ],
  };

  return JSON.stringify(sarif, null, 2);
};

export type OutputFormat = "console" | "json" | "sarif";

export const format = (result: CheckResult, fmt: OutputFormat): string => {
  switch (fmt) {
    case "json":
      return formatJson(result);
    case "sarif":
      return formatSarif(result);
    default:
      return formatConsole(result);
  }
};

export const formatEffect = (
  result: CheckResult,
  fmt: OutputFormat,
): Effect.Effect<string> => Effect.succeed(format(result, fmt));
