# Local CI/CD commands - run these before pushing to GitHub
# Install just: brew install just (macOS) or cargo install just

# Default recipe shows available commands
default:
    @just --list

# Check if Docker is available
_check-docker:
    @command -v docker >/dev/null 2>&1 || (echo "❌ Docker not found. Install Docker Desktop to run Linux checks." && exit 1)

# Install all required Rust tools
install-tools:
    @echo "📦 Installing required Rust tools..."
    cargo install cargo-release git-cliff cargo-audit cargo-outdated cargo-machete --locked
    @echo "✅ All tools installed"

# Check code formatting
fmt-check:
    @echo "🎨 Checking code formatting..."
    cargo fmt -- --check

# Fix code formatting
fmt:
    @echo "🎨 Fixing code formatting..."
    cargo fmt

# Run Clippy lints (commented out in your workflow, but here if you want it)
clippy:
    @echo "📎 Running Clippy lints..."
    cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -W clippy::pedantic -W clippy::nursery

# Check for unused dependencies
check-unused:
    @echo "🔍 Checking for unused dependencies..."
    cargo machete

# Run security audit
audit:
    @echo "🔒 Running security audit..."
    cargo audit

# Check for outdated dependencies (non-blocking)
check-outdated:
    @echo "📦 Checking for outdated dependencies..."
    -cargo outdated --exit-code 1 || echo "⚠️  Some dependencies are outdated (not blocking)"

# Validate Cargo.toml
validate-cargo:
    @echo "📋 Validating Cargo.toml..."
    cargo metadata --format-version 1 > /dev/null
    @if ! grep -q '^description = ' Cargo.toml; then echo "⚠️  Consider adding a description field"; fi
    @if ! grep -q '^license = ' Cargo.toml && ! grep -q '^license-file = ' Cargo.toml; then echo "⚠️  Consider adding a license field"; fi
    @if ! grep -q '^repository = ' Cargo.toml; then echo "⚠️  Consider adding a repository field"; fi
    @echo "✅ Cargo.toml validation complete"

# Check compilation with different feature combinations
check-features:
    @echo "🔧 Checking compilation with different features..."
    @echo "  → Default features..."
    cargo check --all-targets
    @echo "  → No default features..."
    cargo check --no-default-features --all-targets
    @echo "  → All features..."
    cargo check --all-features --all-targets
    @echo "✅ All feature combinations compile"

# Check documentation generation
check-docs:
    @echo "📚 Checking documentation generation..."
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
    @echo "✅ Documentation generates without warnings"

# Run all quality checks (this is what you want before a PR)
quality: fmt-check check-unused audit check-outdated validate-cargo check-features check-docs
    @echo ""
    @echo "🎉 All quality checks passed!"
    @echo ""
    @echo "Quality checks completed:"
    @echo "  ✅ Code formatting"
    @echo "  ✅ Unused dependencies"
    @echo "  ✅ Security audit"
    @echo "  ✅ Outdated dependencies"
    @echo "  ✅ Cargo.toml validation"
    @echo "  ✅ Feature combinations"
    @echo "  ✅ Documentation"

# Run tests if they exist
test:
    @echo "🧪 Running tests..."
    @if find . -name "*.rs" -path "*/tests/*" -o -name "lib.rs" -exec grep -l "#\[cfg(test)\]" {} \; | head -1 | grep -q .; then \
        cargo test --release; \
    else \
        echo "ℹ️  No tests found, skipping"; \
    fi

# Build release binary
build:
    @echo "🔨 Building release binary..."
    @echo "  → Checking all targets first..."
    cargo check --all-targets --release
    @echo "  → Building release binary..."
    cargo build --release
    @echo "✅ Build complete: target/release/setup-devbox"

# Run advanced checks (non-blocking)
advanced-checks:
    @echo "🔬 Running advanced checks..."
    @echo "  → Analyzing binary size..."
    cargo build --release
    @ls -lah target/release/setup-devbox
    @echo "  → Checking benchmarks..."
    @if find . -name "*.rs" -exec grep -l "#\[bench\]" {} \; | head -1 | grep -q .; then \
        cargo bench --no-run; \
        echo "✅ Benchmarks compile"; \
    else \
        echo "ℹ️  No benchmarks found"; \
    fi
    @echo "✅ Advanced checks complete"

# Full pre-PR check (quality + test + build)
pre-pr: quality test build
    @echo ""
    @echo "✨ Ready to create your PR!"
    @echo ""
    @echo "Everything looks good:"
    @echo "  ✅ All quality checks passed"
    @echo "  ✅ Tests passed"
    @echo "  ✅ Release build successful"

# Quick check (just formatting and basic compilation)
quick: fmt-check
    @echo "⚡ Running quick checks..."
    cargo check --all-targets
    @echo "✅ Quick checks passed"

# Fix common issues automatically
fix: fmt
    @echo "🔧 Auto-fixing issues..."
    @echo "  → Running cargo fix..."
    cargo fix --allow-dirty --allow-staged
    @echo "✅ Auto-fixes applied (review changes with git diff)"

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean
    @echo "✅ Clean complete"

# ============================================================================
# LINUX/DOCKER CHECKS (to match Ubuntu CI environment)
# ============================================================================

# Check compilation on Linux using Docker (matches CI environment)
check-linux: _check-docker
    @echo "🐧 Running Linux checks in Docker (matching CI environment)..."
    @echo "  → Pulling Rust Docker image..."
    @docker pull rust:latest
    @echo "  → Running cargo check on Linux..."
    @docker run --rm -v $(pwd):/workspace -w /workspace rust:latest \
        bash -c "cargo check --all-targets --all-features"
    @echo "✅ Linux checks passed"

# Run all quality checks on Linux using Docker
quality-linux: _check-docker
    @echo "🐧 Running full quality suite on Linux..."
    @docker pull rust:latest
    @docker run --rm -v $(pwd):/workspace -w /workspace rust:latest \
        bash -c "\
            echo '🎨 Checking formatting...' && \
            rustup component add rustfmt && \
            cargo fmt -- --check && \
            echo '🔧 Checking features...' && \
            cargo check --all-targets && \
            cargo check --no-default-features --all-targets && \
            cargo check --all-features --all-targets && \
            echo '📚 Checking docs...' && \
            RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --document-private-items && \
            echo '✅ All Linux quality checks passed!'"

# Full pre-PR check including Linux verification
pre-pr-with-linux: quality test build check-linux
    @echo ""
    @echo "✨ Ready to create your PR (Linux verified)!"
    @echo ""
    @echo "Everything looks good:"
    @echo "  ✅ All quality checks passed"
    @echo "  ✅ Tests passed"
    @echo "  ✅ Release build successful"
    @echo "  ✅ Linux compilation verified"

# ============================================================================
# RELEASE COMMANDS (standalone - don't run these casually!)
# ============================================================================

# Analyze what version bump is needed
analyze-version:
    #!/usr/bin/env bash
    set -euo pipefail

    current_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    echo "Current version: $current_version"

    last_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
    if [[ -z "$last_tag" ]]; then
        echo "No previous tags found - this will be initial release"
        echo "Suggested: minor bump"
        exit 0
    fi

    echo "Last tag: $last_tag"
    echo ""
    echo "Commits since last release:"
    git log ${last_tag}..HEAD --oneline
    echo ""

    if git log ${last_tag}..HEAD --grep="BREAKING CHANGE" | grep -q "BREAKING CHANGE"; then
        echo "⚠️  Breaking changes detected → MAJOR bump needed"
    elif git log ${last_tag}..HEAD --oneline | grep -q "feat:"; then
        echo "✨ Features detected → MINOR bump suggested"
    elif git log ${last_tag}..HEAD --oneline | grep -qE "fix:|perf:|refactor:"; then
        echo "🐛 Patches/fixes detected → PATCH bump suggested"
    else
        echo "ℹ️  No conventional commits found"
    fi

# Create a new release (YOU MUST SPECIFY: just release major|minor|patch)
release bump_type:
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ "{{bump_type}}" != "major" && "{{bump_type}}" != "minor" && "{{bump_type}}" != "patch" ]]; then
        echo "❌ Invalid bump type. Use: just release major|minor|patch"
        exit 1
    fi

    echo "🚀 Creating {{bump_type}} release..."
    echo ""

    # Make sure we're on main or development
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [[ "$branch" != "main" && "$branch" != "development" ]]; then
        echo "❌ Releases must be created from 'main' or 'development' branch"
        echo "   Current branch: $branch"
        exit 1
    fi

    # Make sure working directory is clean
    if [[ -n $(git status --porcelain) ]]; then
        echo "❌ Working directory is not clean. Commit or stash your changes first."
        git status --short
        exit 1
    fi

    # Update version
    echo "📝 Updating version in Cargo.toml..."
    cargo release version --execute {{bump_type}} --no-confirm

    new_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    echo "New version: $new_version"

    # Generate changelog
    echo "📋 Generating changelog..."
    git-cliff --tag v${new_version} > CHANGELOG.md

    # Commit changes
    echo "💾 Committing version bump and changelog..."
    git add Cargo.toml Cargo.lock CHANGELOG.md
    git commit -m "chore(release): ${new_version} [skip ci]"

    # Create tag
    echo "🏷️  Creating tag v${new_version}..."
    git tag -a "v${new_version}" -m "Release v${new_version}"

    echo ""
    echo "✅ Release v${new_version} prepared!"
    echo ""
    echo "Next steps:"
    echo "  1. Review the changes: git log -1 && git show v${new_version}"
    echo "  2. Push to GitHub: git push origin $branch && git push origin v${new_version}"
    echo "  3. GitHub Actions will create the release automatically"
    echo ""
    echo "To undo: git reset --hard HEAD~1 && git tag -d v${new_version}"

# Preview what the next release would look like
preview-release:
    @echo "📋 Preview of next release changelog:"
    @echo ""
    git-cliff --unreleased

# View current changelog
changelog:
    @echo "📖 Current changelog:"
    @echo ""
    @if [ -f CHANGELOG.md ]; then cat CHANGELOG.md; else echo "No CHANGELOG.md found"; fi
