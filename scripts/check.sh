#!/bin/bash
# A local script to run the exact same checks as the GitHub Actions CI pipeline

set -e # Exit immediately if a command exits with a non-zero status.

echo "========================================="
echo "🚀 Running Local CI Checks..."
echo "========================================="

echo ""
echo "🦀 1. Formatting Rust Code (cargo fmt)..."
cargo fmt --all

echo ""
echo "🦀 2. Linting Rust Code (cargo clippy)..."
cargo clippy --workspace -- -D warnings

echo ""
echo "🦀 3. Testing Rust Backend (cargo test)..."
cargo test --workspace

echo ""
echo "⚛️  4. Typechecking React Frontend (npm run typecheck)..."
cd web
npm run typecheck

echo ""
echo "⚛️  5. Verifying React Build (npm run build)..."
npm run build
cd ..

echo ""
echo "========================================="
echo "✅ All checks passed successfully! You are ready to commit and push."
echo "========================================="
