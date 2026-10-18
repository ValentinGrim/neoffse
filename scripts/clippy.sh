#!/usr/bin/env bash
# Strict clippy gate for the whole workspace. Any warning is a failure.
# The lint set lives in the root Cargo.toml under [workspace.lints], so
# rust-analyzer and the agents' LSP see the same rules this gate enforces.
# Per-lint exceptions belong in the code as #[allow(...)] with a one-line
# justification, never as extra flags here.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo clippy --workspace --all-targets --all-features -- -D warnings
