#!/usr/bin/env bash
# scripts/pre-commit.sh
#
# Pre-commit hook: auto-formats staged Rust files with cargo fmt.
#
# Install as a git hook by running:
#   cp scripts/pre-commit.sh .git/hooks/pre-commit
#   chmod +x .git/hooks/pre-commit

set -euo pipefail

echo ""
echo "========================================="
echo "📝 Running Pre-Commit Checks..."
echo "========================================="

echo ""
echo "🦀 Formatting Rust code (cargo fmt)..."

# Format all Rust code in the workspace
cargo fmt --all

# Find all Rust files that are currently staged for this commit
STAGED_RUST_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

# If cargo fmt modified any of the staged files, re-add them so the
# formatted version is what actually gets committed (not the pre-format version)
if [ -n "$STAGED_RUST_FILES" ]; then
    echo "$STAGED_RUST_FILES" | xargs git add
    echo "   ↳ Re-staged formatted files."
fi

echo ""
echo "========================================="
echo "✅ Pre-commit checks passed. Committing."
echo "========================================="
echo ""

exit 0
