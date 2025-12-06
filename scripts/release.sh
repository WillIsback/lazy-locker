#!/bin/bash
# Prepare and create a release
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.0.5

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

if [ -z "$1" ]; then
    echo "Usage: ./scripts/release.sh <version>"
    echo "Example: ./scripts/release.sh 0.0.5"
    exit 1
fi

VERSION="$1"
TAG="v$VERSION"

echo -e "${BLUE}🚀 Preparing release $TAG${NC}"
echo ""

# ============================================================================
# 1. PRE-FLIGHT CHECKS
# ============================================================================
echo -e "${YELLOW}1. Running pre-flight checks...${NC}"

# Check we're on master/main
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "master" ] && [ "$BRANCH" != "main" ]; then
    echo -e "${RED}❌ Must be on master or main branch (current: $BRANCH)${NC}"
    exit 1
fi

# Check working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${RED}❌ Working directory is not clean. Commit or stash changes first.${NC}"
    exit 1
fi

# Check tag doesn't exist
if git tag -l | grep -q "^$TAG$"; then
    echo -e "${RED}❌ Tag $TAG already exists${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Pre-flight checks passed${NC}"

# ============================================================================
# 2. BUMP VERSION
# ============================================================================
echo -e "${YELLOW}2. Bumping version...${NC}"
./scripts/bump-version.sh "$VERSION"

# ============================================================================
# 3. VERIFY CHANGELOG
# ============================================================================
echo -e "${YELLOW}3. Checking CHANGELOG...${NC}"
if ! grep -q "\[$VERSION\]" CHANGELOG.md; then
    echo -e "${RED}❌ Version $VERSION not found in CHANGELOG.md${NC}"
    echo "Please add a changelog entry for this version."
    echo ""
    echo "Opening CHANGELOG.md..."
    ${EDITOR:-vim} CHANGELOG.md
    
    if ! grep -q "\[$VERSION\]" CHANGELOG.md; then
        echo -e "${RED}❌ Still no changelog entry. Aborting.${NC}"
        exit 1
    fi
fi
echo -e "${GREEN}✅ CHANGELOG has entry for $VERSION${NC}"

# ============================================================================
# 4. FORMAT AND LINT CHECKS
# ============================================================================
echo -e "${YELLOW}4. Checking code formatting...${NC}"
if ! cargo fmt --check; then
    echo -e "${RED}❌ Code is not formatted. Run 'cargo fmt' first.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Formatting OK${NC}"

echo -e "${YELLOW}5. Running clippy...${NC}"
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${RED}❌ Clippy found issues. Please fix them before releasing.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Clippy OK${NC}"

# ============================================================================
# 6. RUN TESTS
# ============================================================================
echo -e "${YELLOW}6. Running tests...${NC}"
cargo test --all-features
echo -e "${GREEN}✅ Tests passed${NC}"

# ============================================================================
# 7. BUILD RELEASE
# ============================================================================
echo -e "${YELLOW}7. Building release...${NC}"
cargo build --release
echo -e "${GREEN}✅ Build successful${NC}"

# ============================================================================
# 8. COMMIT AND TAG
# ============================================================================
echo -e "${YELLOW}8. Creating commit and tag...${NC}"
git add -A
git commit -m "chore: release v$VERSION"
git tag -a "$TAG" -m "Release $TAG"
echo -e "${GREEN}✅ Created commit and tag $TAG${NC}"

# ============================================================================
# 9. PUSH
# ============================================================================
echo ""
echo -e "${BLUE}Ready to push!${NC}"
echo ""
echo "This will push to origin and trigger the release workflow."
echo "The workflow will publish to:"
echo "  - crates.io"
echo "  - PyPI"
echo "  - npm"
echo "  - GitHub Releases"
echo ""
read -p "Push now? [y/N] " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push origin "$BRANCH"
    git push origin "$TAG"
    echo ""
    echo -e "${GREEN}🎉 Release $TAG pushed!${NC}"
    echo ""
    echo "Monitor the release workflow:"
    echo "  https://github.com/WillIsback/lazy-locker/actions"
else
    echo ""
    echo -e "${YELLOW}Push cancelled. To push manually:${NC}"
    echo "  git push origin $BRANCH"
    echo "  git push origin $TAG"
fi
