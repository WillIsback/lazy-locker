#!/bin/bash
# Bump version across all project files
# Usage: ./scripts/bump-version.sh <new_version>

set -e

if [ -z "$1" ]; then
    echo "Usage: ./scripts/bump-version.sh <new_version>"
    echo "Example: ./scripts/bump-version.sh 0.0.5"
    exit 1
fi

NEW_VERSION="$1"

# Validate version format (x.y.z)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "❌ Invalid version format. Use semantic versioning: x.y.z"
    exit 1
fi

echo "🔄 Bumping version to $NEW_VERSION..."

# Update Cargo.toml
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
echo "✅ Updated Cargo.toml"

# Update Python SDK
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" sdk/python/pyproject.toml
echo "✅ Updated sdk/python/pyproject.toml"

# Update JavaScript SDK
sed -i "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" sdk/javascript/package.json
echo "✅ Updated sdk/javascript/package.json"

# Verify
echo ""
echo "📋 Version check:"
echo "   Cargo.toml: $(grep '^version' Cargo.toml | head -1)"
echo "   Python SDK: $(grep '^version' sdk/python/pyproject.toml | head -1)"
echo "   JS SDK:     $(grep '"version"' sdk/javascript/package.json | head -1)"

echo ""
echo "🎉 Version bumped to $NEW_VERSION"
echo ""
echo "Next steps:"
echo "  1. Update CHANGELOG.md with new version entry"
echo "  2. git add -A && git commit -m 'chore: bump version to $NEW_VERSION'"
echo "  3. git tag v$NEW_VERSION"
echo "  4. git push && git push --tags"
