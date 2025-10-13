# Local CI/CD commands - run these before pushing to GitHub
# Usage: make <target>

.PHONY: help install-tools fmt-check fmt clippy check-unused audit check-outdated
.PHONY: validate-cargo check-features check-docs quality test build advanced-checks
.PHONY: pre-pr quick fix clean analyze-version release preview-release changelog
.PHONY: check-linux quality-linux pre-pr-with-linux _check-docker

# Default target shows help
help:
	@echo "Available targets:"
	@echo ""
	@echo "Setup:"
	@echo "  install-tools     - Install required Rust tools"
	@echo ""
	@echo "Quality Checks:"
	@echo "  fmt-check        - Check code formatting"
	@echo "  fmt              - Fix code formatting"
	@echo "  clippy           - Run Clippy lints"
	@echo "  check-unused     - Check for unused dependencies"
	@echo "  audit            - Run security audit"
	@echo "  check-outdated   - Check for outdated dependencies"
	@echo "  validate-cargo   - Validate Cargo.toml"
	@echo "  check-features   - Check feature combinations"
	@echo "  check-docs       - Check documentation"
	@echo "  quality          - Run all quality checks"
	@echo ""
	@echo "Build & Test:"
	@echo "  test             - Run tests"
	@echo "  build            - Build release binary"
	@echo "  advanced-checks  - Run advanced analysis"
	@echo ""
	@echo "Convenience:"
	@echo "  pre-pr           - Full pre-PR check (quality + test + build)"
	@echo "  quick            - Quick check (fmt + basic compile)"
	@echo "  fix              - Auto-fix issues"
	@echo "  clean            - Clean build artifacts"
	@echo ""
	@echo "Linux/Docker Checks:"
	@echo "  check-linux      - Check compilation on Linux (Docker)"
	@echo "  quality-linux    - Run quality checks on Linux (Docker)"
	@echo "  pre-pr-with-linux - Full pre-PR check + Linux verification"
	@echo ""
	@echo "Release (standalone):"
	@echo "  analyze-version  - Analyze what version bump is needed"
	@echo "  release-major    - Create major release"
	@echo "  release-minor    - Create minor release"
	@echo "  release-patch    - Create patch release"
	@echo "  preview-release  - Preview next release changelog"
	@echo "  changelog        - View current changelog"

# Install all required Rust tools
install-tools:
	@echo "📦 Installing required Rust tools..."
	@cargo install cargo-release git-cliff cargo-audit cargo-outdated cargo-machete --locked
	@echo "✅ All tools installed"

# Check code formatting
fmt-check:
	@echo "🎨 Checking code formatting..."
	@cargo fmt -- --check

# Fix code formatting
fmt:
	@echo "🎨 Fixing code formatting..."
	@cargo fmt

# Run Clippy lints
clippy:
	@echo "📎 Running Clippy lints..."
	@cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -W clippy::pedantic -W clippy::nursery

# Check for unused dependencies
check-unused:
	@echo "🔍 Checking for unused dependencies..."
	@cargo machete

# Run security audit
audit:
	@echo "🔒 Running security audit..."
	@cargo audit

# Check for outdated dependencies (non-blocking)
check-outdated:
	@echo "📦 Checking for outdated dependencies..."
	@cargo outdated --exit-code 1 || echo "⚠️  Some dependencies are outdated (not blocking)"

# Validate Cargo.toml
validate-cargo:
	@echo "📋 Validating Cargo.toml..."
	@cargo metadata --format-version 1 > /dev/null
	@grep -q '^description = ' Cargo.toml || echo "⚠️  Consider adding a description field"
	@grep -q '^license = ' Cargo.toml || grep -q '^license-file = ' Cargo.toml || echo "⚠️  Consider adding a license field"
	@grep -q '^repository = ' Cargo.toml || echo "⚠️  Consider adding a repository field"
	@echo "✅ Cargo.toml validation complete"

# Check compilation with different feature combinations
check-features:
	@echo "🔧 Checking compilation with different features..."
	@echo "  → Default features..."
	@cargo check --all-targets
	@echo "  → No default features..."
	@cargo check --no-default-features --all-targets
	@echo "  → All features..."
	@cargo check --all-features --all-targets
	@echo "✅ All feature combinations compile"

# Check documentation generation
check-docs:
	@echo "📚 Checking documentation generation..."
	@RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
	@echo "✅ Documentation generates without warnings"

# Run all quality checks
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
	@cargo check --all-targets --release
	@echo "  → Building release binary..."
	@cargo build --release
	@echo "✅ Build complete: target/release/setup-devbox"

# Run advanced checks (non-blocking)
advanced-checks:
	@echo "🔬 Running advanced checks..."
	@echo "  → Analyzing binary size..."
	@cargo build --release
	@ls -lah target/release/setup-devbox
	@echo "  → Checking benchmarks..."
	@if find . -name "*.rs" -exec grep -l "#\[bench\]" {} \; | head -1 | grep -q .; then \
		cargo bench --no-run && echo "✅ Benchmarks compile"; \
	else \
		echo "ℹ️  No benchmarks found"; \
	fi
	@echo "✅ Advanced checks complete"

# Full pre-PR check
pre-pr: quality test build
	@echo ""
	@echo "✨ Ready to create your PR!"
	@echo ""
	@echo "Everything looks good:"
	@echo "  ✅ All quality checks passed"
	@echo "  ✅ Tests passed"
	@echo "  ✅ Release build successful"

# Quick check
quick: fmt-check
	@echo "⚡ Running quick checks..."
	@cargo check --all-targets
	@echo "✅ Quick checks passed"

# Fix common issues automatically
fix: fmt
	@echo "🔧 Auto-fixing issues..."
	@echo "  → Running cargo fix..."
	@cargo fix --allow-dirty --allow-staged
	@echo "✅ Auto-fixes applied (review changes with git diff)"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean complete"

# ============================================================================
# LINUX/DOCKER CHECKS (to match Ubuntu CI environment)
# ============================================================================

# Internal: Check if Docker is available
_check-docker:
	@command -v docker >/dev/null 2>&1 || (echo "❌ Docker not found. Install Docker Desktop to run Linux checks." && exit 1)

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
	@current_version=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	echo "Current version: $$current_version"; \
	echo ""; \
	last_tag=$$(git describe --tags --abbrev=0 2>/dev/null || echo ""); \
	if [ -z "$$last_tag" ]; then \
		echo "No previous tags found - this will be initial release"; \
		echo "Suggested: minor bump"; \
		exit 0; \
	fi; \
	echo "Last tag: $$last_tag"; \
	echo ""; \
	echo "Commits since last release:"; \
	git log $$last_tag..HEAD --oneline; \
	echo ""; \
	if git log $$last_tag..HEAD --grep="BREAKING CHANGE" | grep -q "BREAKING CHANGE"; then \
		echo "⚠️  Breaking changes detected → MAJOR bump needed"; \
	elif git log $$last_tag..HEAD --oneline | grep -q "feat:"; then \
		echo "✨ Features detected → MINOR bump suggested"; \
	elif git log $$last_tag..HEAD --oneline | grep -qE "fix:|perf:|refactor:"; then \
		echo "🐛 Patches/fixes detected → PATCH bump suggested"; \
	else \
		echo "ℹ️  No conventional commits found"; \
	fi

# Internal release helper (don't call directly)
_do-release:
	@if [ -z "$(BUMP_TYPE)" ]; then \
		echo "❌ Error: Use 'make release-major', 'make release-minor', or 'make release-patch'"; \
		exit 1; \
	fi; \
	echo "🚀 Creating $(BUMP_TYPE) release..."; \
	echo ""; \
	branch=$$(git rev-parse --abbrev-ref HEAD); \
	if [ "$$branch" != "main" ] && [ "$$branch" != "development" ]; then \
		echo "❌ Releases must be created from 'main' or 'development' branch"; \
		echo "   Current branch: $$branch"; \
		exit 1; \
	fi; \
	if [ -n "$$(git status --porcelain)" ]; then \
		echo "❌ Working directory is not clean. Commit or stash your changes first."; \
		git status --short; \
		exit 1; \
	fi; \
	echo "📝 Updating version in Cargo.toml..."; \
	cargo release version --execute $(BUMP_TYPE) --no-confirm; \
	new_version=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	echo "New version: $$new_version"; \
	echo "📋 Generating changelog..."; \
	git-cliff --tag v$$new_version > CHANGELOG.md; \
	echo "💾 Committing version bump and changelog..."; \
	git add Cargo.toml Cargo.lock CHANGELOG.md; \
	git commit -m "chore(release): $$new_version [skip ci]"; \
	echo "🏷️  Creating tag v$$new_version..."; \
	git tag -a "v$$new_version" -m "Release v$$new_version"; \
	echo ""; \
	echo "✅ Release v$$new_version prepared!"; \
	echo ""; \
	echo "Next steps:"; \
	echo "  1. Review the changes: git log -1 && git show v$$new_version"; \
	echo "  2. Push to GitHub: git push origin $$branch && git push origin v$$new_version"; \
	echo "  3. GitHub Actions will create the release automatically"; \
	echo ""; \
	echo "To undo: git reset --hard HEAD~1 && git tag -d v$$new_version"

# Create major release
release-major:
	@$(MAKE) _do-release BUMP_TYPE=major

# Create minor release
release-minor:
	@$(MAKE) _do-release BUMP_TYPE=minor

# Create patch release
release-patch:
	@$(MAKE) _do-release BUMP_TYPE=patch

# Preview what the next release would look like
preview-release:
	@echo "📋 Preview of next release changelog:"
	@echo ""
	@git-cliff --unreleased

# View current changelog
changelog:
	@echo "📖 Current changelog:"
	@echo ""
	@if [ -f CHANGELOG.md ]; then cat CHANGELOG.md; else echo "No CHANGELOG.md found"; fi
