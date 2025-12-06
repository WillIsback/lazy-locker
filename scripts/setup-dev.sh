#!/bin/bash
# Setup development environment
# Usage: ./scripts/setup-dev.sh

set -e

echo "🔧 Setting up development environment..."
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ============================================================================
# 1. Install pre-commit hook
# ============================================================================
echo "📋 Installing pre-commit hook..."
if [ -d ".git" ]; then
    cp scripts/pre-commit.sh .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    echo -e "${GREEN}✅ Pre-commit hook installed${NC}"
else
    echo -e "${YELLOW}⚠️  Not a git repository, skipping pre-commit hook${NC}"
fi

# ============================================================================
# 2. Check Rust toolchain
# ============================================================================
echo ""
echo "🦀 Checking Rust toolchain..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}✅ Rust installed: $RUST_VERSION${NC}"
else
    echo -e "${YELLOW}⚠️  Rust not found. Install from https://rustup.rs${NC}"
fi

if command -v cargo &> /dev/null; then
    # Check for clippy and rustfmt
    if rustup component list --installed | grep -q clippy; then
        echo -e "${GREEN}✅ Clippy installed${NC}"
    else
        echo "Installing clippy..."
        rustup component add clippy
    fi
    
    if rustup component list --installed | grep -q rustfmt; then
        echo -e "${GREEN}✅ Rustfmt installed${NC}"
    else
        echo "Installing rustfmt..."
        rustup component add rustfmt
    fi
fi

# ============================================================================
# 3. Build project
# ============================================================================
echo ""
echo "🔨 Building project..."
cargo build
echo -e "${GREEN}✅ Build successful${NC}"

# ============================================================================
# 4. Run tests
# ============================================================================
echo ""
echo "🧪 Running tests..."
cargo test --lib
echo -e "${GREEN}✅ Tests passed${NC}"

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "=========================================="
echo -e "${GREEN}✅ Development environment ready!${NC}"
echo "=========================================="
echo ""
echo "Available scripts:"
echo "  ./scripts/pre-commit.sh    - Run pre-commit checks manually"
echo "  ./scripts/bump-version.sh  - Bump version across all files"
echo "  ./scripts/release.sh       - Prepare and push a release"
echo ""
echo "Workflows:"
echo "  .github/workflows/ci.yml      - CI on push/PR"
echo "  .github/workflows/release.yml - Release on tag push"
echo ""
