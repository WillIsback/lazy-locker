#!/bin/bash
# Pre-commit hook for lazy-locker
# Install: cp scripts/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit

set -e

echo "🔍 Running pre-commit checks..."

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Must run from project root${NC}"
    exit 1
fi

# ============================================================================
# 1. FORMATTING CHECK (cargo fmt)
# ============================================================================
echo "📐 Checking code formatting..."
if ! cargo fmt --check 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Code is not formatted. Running cargo fmt...${NC}"
    cargo fmt
    echo -e "${GREEN}✅ Code formatted. Please review and re-add changed files.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Formatting OK${NC}"

# ============================================================================
# 2. LINTING (cargo clippy)
# ============================================================================
echo "🔎 Running clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings 2>/dev/null; then
    echo -e "${RED}❌ Clippy found issues. Please fix them before committing.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Clippy OK${NC}"

# ============================================================================
# 3. BUILD CHECK
# ============================================================================
echo "🔨 Checking build..."
if ! cargo build --release 2>/dev/null; then
    echo -e "${RED}❌ Build failed. Please fix before committing.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Build OK${NC}"

# ============================================================================
# 4. VERSION CONSISTENCY CHECK
# ============================================================================
echo "🔢 Checking version consistency..."

# Extract versions
CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
PYTHON_VERSION=$(grep '^version' sdk/python/pyproject.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
JS_VERSION=$(grep '"version"' sdk/javascript/package.json | head -1 | sed 's/.*"\([0-9.]*\)".*/\1/')

if [ "$CARGO_VERSION" != "$PYTHON_VERSION" ] || [ "$CARGO_VERSION" != "$JS_VERSION" ]; then
    echo -e "${RED}❌ Version mismatch detected:${NC}"
    echo "   Cargo.toml:              $CARGO_VERSION"
    echo "   sdk/python/pyproject.toml: $PYTHON_VERSION"  
    echo "   sdk/javascript/package.json: $JS_VERSION"
    echo ""
    echo "Run: ./scripts/bump-version.sh <new_version>"
    exit 1
fi
echo -e "${GREEN}✅ Versions consistent: $CARGO_VERSION${NC}"

# ============================================================================
# 5. CHANGELOG CHECK
# ============================================================================
echo "📝 Checking CHANGELOG..."
if ! grep -q "\[$CARGO_VERSION\]" CHANGELOG.md; then
    echo -e "${YELLOW}⚠️  Warning: Version $CARGO_VERSION not found in CHANGELOG.md${NC}"
    echo "   Consider adding an entry before committing."
fi
echo -e "${GREEN}✅ CHANGELOG check complete${NC}"

echo ""
echo -e "${GREEN}✅ All pre-commit checks passed!${NC}"
