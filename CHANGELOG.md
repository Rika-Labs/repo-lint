# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-01-18

### Removed

- **BREAKING**: Removed unimplemented `boundaries` rule from schema ([#6](https://github.com/Rika-Labs/repo-lint/issues/6))
  - The `BoundaryRule` schema was exported but had no implementation
  - This caused silent failures where users could configure boundaries but nothing would happen
  - Removed from schema, config loader, and inspect command to prevent confusion

### Fixed

- **BREAKING**: Fixed glob pattern matching so `*` no longer matches across path separators ([#3](https://github.com/Rika-Labs/repo-lint/pull/3))
  - Previously, `modules/*` would incorrectly match `modules/chat/stream`
  - Now, `modules/*` only matches `modules/chat` (single segment)
  - Use `**` to match across path separators: `modules/**` matches `modules/chat/stream`

- Basename-only glob patterns (e.g., `*.log`, `*.d.ts`) are auto-expanded to match anywhere
  - `*.log` automatically becomes `**/*.log` so it matches `src/debug.log`
  - This preserves intuitive behavior for `ignore` and `forbidPaths` configs
  - Patterns with `/` are NOT expanded (e.g., `src/*.ts` stays as-is)
  - Literal patterns without glob chars are NOT expanded (e.g., `package.json`)

- Unicode paths now match correctly regardless of NFC/NFD normalization
  - `café.ts` (composed) now matches `café.ts` (decomposed)

- `joinPath()` now normalizes its output (Windows backslashes → forward slashes)

- `expandBraces()` now throws an error on nested braces instead of returning garbage patterns

### Changed

- All matcher functions now normalize Windows paths (backslashes → forward slashes)
- All paths and patterns are now Unicode-normalized to NFC form
- Added LRU-style matcher cache with 1000 pattern limit (prevents memory leaks)
- `matchesWithBraces` now uses the same options as `matches` for consistent behavior

### Added

- `clearMatcherCache()` function for testing and cache invalidation
- `getMatcherCacheSize()` function for monitoring cache size
- `getMaxCacheSize()` function to get the cache limit
- `normalizePath()` is now exported for external use
- Comprehensive JSDoc documentation for all matcher functions

### Migration Guide

If you have patterns that relied on `*` matching across `/`, update them to use `**`:

```yaml
# Before (v1.1.0 and earlier)
match:
  - pattern: "src/*"  # This matched src/a/b/c incorrectly

# After (v2.0.0+)
match:
  - pattern: "src/**"  # Use ** to match across directories
  # OR
  - pattern: "src/*"   # Now correctly matches only src/a, src/b, etc.
```

If you used nested braces in patterns, split them into multiple patterns:

```yaml
# Before (would silently produce garbage)
pattern: "*.{ts,{js,jsx}}"

# After (explicit and correct)
patterns:
  - "*.ts"
  - "*.js" 
  - "*.jsx"
```

## [1.1.0] - 2026-01-17

### Added

- Match-based rules for flexible directory structure validation
- `case` option to validate matched directory naming
- `childCase` option to validate children naming
- Hidden file exemption from case validation
- Case suggestions in violation messages
- Violation deduplication for overlapping rules

### Fixed

- Empty `allowedPatterns` in strict mode now rejects all entries (not allows all)

## [1.0.1] - 2026-01-17

### Fixed

- CLI entry point path in package.json

## [1.0.0] - 2026-01-17

### Added

- Initial release
- Layout-based directory structure validation
- Forbidden paths and names rules
- Dependency/mirror rules
- YAML configuration with extends support
- Multiple output formats (console, JSON, SARIF)
- Caching for improved performance
- Next.js preset
