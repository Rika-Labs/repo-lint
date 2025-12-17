# Installation

## From Cargo (Recommended)

```bash
cargo install repo-lint
```

## From Source

```bash
git clone https://github.com/rika-labs/repo-lint.git
cd repo-lint
cargo build --release
```

The binary will be at `target/release/repo-lint`.

## Verify Installation

```bash
repo-lint --version
```

## Shell Completion

Generate shell completions:

```bash
# Bash
repo-lint completions bash > ~/.local/share/bash-completion/completions/repo-lint

# Zsh
repo-lint completions zsh > ~/.zfunc/_repo-lint

# Fish
repo-lint completions fish > ~/.config/fish/completions/repo-lint.fish
```
