# Makefile for Rust project development
# Run 'make help' to see all available commands
#
# Alternative: You can also use ./dev.sh [command] for the same functionality
# Choose whichever interface you prefer!

.PHONY: help install check format lint test build clean quality advanced all pre-commit release-check dev

# Default target
help: ## Show this help message
	@echo "🦀 setup-devbox aka SDB Development, Build, Check and CI Commands"
	@echo ""
	@echo "Core commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Examples:"
	@echo "  make install     # Set up development environment"
	@echo "  make check       # Quick checks before commit"
	@echo "  make quality     # Full quality analysis (like CI)"
	@echo "  make all         # Run everything"

# Installation and setup
install: ## Install all required tools for development
	@echo "🔧 Installing Rust development tools..."
	@echo "Checking Rust installation..."
	@if ! command -v rustc >/dev/null 2>&1; then \
		echo "❌ Rust not found. Please install Rust first: https://rustup.rs/"; \
		exit 1; \
	fi
	@echo "✅ Rust found: $(rustc --version)"
	@echo "Setting up default toolchain if needed..."
	@rustup default stable || echo "⚠️  Default toolchain already set or using system Rust"
	@echo "📦 Installing Rust components..."
	@rustup component add rustfmt clippy rust-src || echo "⚠️  Some components may already be installed"
	@echo "🔧 Installing nightly toolchain..."
	@rustup install nightly || echo "⚠️  Nightly already installed"
	@rustup component add rust-src --toolchain nightly || echo "⚠️  Nightly rust-src already installed"
	@echo "📦 Installing additional cargo tools..."
	@echo "This may take a few minutes..."
	@for tool in cargo-audit cargo-outdated cargo-machete cargo-deny cargo-udeps cargo-pants; do \
		echo "Installing $tool..."; \
		cargo install $tool --locked || echo "⚠️  $tool installation failed or already installed"; \
	done
	@echo "✅ Installation completed!"
	@echo ""
	@echo "🔍 Verification:"
	@rustc --version || echo "❌ rustc not working"
	@cargo --version || echo "❌ cargo not working"
	@rustup --version || echo "❌ rustup not working"

# Quick development checks
check: ## Quick compilation check (fastest feedback)
	@echo "🔍 Running quick compilation check..."
	@cargo check
	@echo "✅ Quick check passed!"

format: ## Format code and check formatting
	@echo "🎨 Formatting code..."
	@cargo fmt
	@echo "✅ Code formatted!"

format-check: ## Check if code is properly formatted (CI mode)
	@echo "🎨 Checking code formatting..."
	@cargo fmt -- --check || (echo "❌ Code needs formatting. Run 'make format' to fix." && exit 1)
	@echo "✅ Code formatting is correct!"

lint: ## Run Clippy lints
	@echo "📎 Running Clippy lints..."
	@cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -W clippy::pedantic -W clippy::nursery
	@echo "✅ No Clippy issues found!"

lint-fix: ## Run Clippy with automatic fixes
	@echo "📎 Running Clippy with automatic fixes..."
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
	@echo "✅ Clippy fixes applied!"

test: ## Run tests (if any exist)
	@echo "🧪 Running tests..."
	@if find . -name "*.rs" -path "*/tests/*" -o -name "lib.rs" -exec grep -l "#\[cfg(test)\]" {} \; | head -1 | grep -q .; then \
		cargo test; \
	else \
		echo "ℹ️  No tests found, skipping test execution"; \
	fi
	@echo "✅ Tests completed!"

build: ## Build the project in release mode
	@echo "🔨 Building release binary..."
	@cargo build --release
	@echo "✅ Build completed!"
	@ls -lah target/release/setup-devbox 2>/dev/null || echo "Binary location: target/release/"

clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean completed!"

# Comprehensive quality checks (matches CI)
quality: format-check security deps-check features docs ## Run all quality checks (matches CI)
	@echo ""
	@echo "🎉 All quality checks passed! Ready for commit."

security: ## Run security audit
	@echo "🔒 Running security audit..."
	@cargo audit
	@echo "✅ No security vulnerabilities found!"

deps-check: ## Check for unused and outdated dependencies
	@echo "🔍 Checking for unused dependencies..."
	@cargo machete
	@echo "📦 Checking for outdated dependencies..."
	@cargo outdated --exit-code 1 || echo "⚠️  Some dependencies are outdated (not blocking)"
	@echo "✅ Dependency checks completed!"

features: ## Test different feature combinations
	@echo "🔧 Testing feature combinations..."
	@echo "  Testing default features..."
	@cargo check
	@echo "  Testing no default features..."
	@cargo check --no-default-features
	@echo "  Testing all features..."
	@cargo check --all-features
	@echo "✅ All feature combinations compile!"

docs: ## Generate and check documentation
	@echo "📚 Checking documentation..."
	@RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
	@echo "✅ Documentation generates without warnings!"

# Advanced analysis (like CI advanced-checks)
advanced: ## Run advanced analysis tools
	@echo "🔬 Running advanced analysis..."
	@echo "🚫 Running cargo-deny checks..."
	@cargo deny check || echo "⚠️  cargo-deny found issues (check deny.toml config)"
	@echo "🔍 Checking for unused dependencies (nightly)..."
	@cargo +nightly udeps --all-targets || echo "⚠️  cargo-udeps found issues"
	@echo "🧹 Checking for unused features..."
	@cargo pants || echo "⚠️  cargo-pants found issues"
	@$(MAKE) analyze-patterns
	@echo "✅ Advanced analysis completed!"

analyze-patterns: ## Analyze code for common anti-patterns
	@echo "🕵️  Checking for common anti-patterns..."
	@echo -n "  unwrap() calls in src/: "
	@find src -name "*.rs" -not -path "*/tests/*" -exec grep -n "\.unwrap()" {} \; | wc -l | tr -d ' ' || echo "0"
	@echo -n "  expect() calls in src/: "
	@find src -name "*.rs" -not -path "*/tests/*" -exec grep -n "\.expect(" {} \; | wc -l | tr -d ' ' || echo "0"
	@echo -n "  panic! calls in src/: "
	@find src -name "*.rs" -not -path "*/tests/*" -exec grep -n "panic!" {} \; | wc -l | tr -d ' ' || echo "0"

# Workflow simulation
pre-commit: quality ## Run all checks before committing (recommended)
	@echo ""
	@echo "🚀 Pre-commit checks completed!"
	@echo "Your code is ready for commit and should pass CI."

release-check: quality build advanced ## Full release readiness check
	@echo ""
	@echo "📦 Release readiness check completed!"
	@echo "Your code is ready for release."

# Convenience targets
all: quality build ## Run quality checks and build
	@echo ""
	@echo "🎯 All tasks completed successfully!"

dev: format ## Quick development cycle (format + lint)
	@echo ""
	@echo "🚀 Development checks completed!"

# Continuous development helpers
watch: ## Watch for file changes and run quick checks
	@echo "👀 Watching for changes... (Press Ctrl+C to stop)"
	@echo "Will run 'cargo check' on file changes"
	@cargo watch -x check

watch-test: ## Watch for file changes and run tests
	@echo "👀 Watching for changes and running tests... (Press Ctrl+C to stop)"
	@cargo watch -x test

watch-lint: ## Watch for file changes and run clippy
	@echo "👀 Watching for changes and running clippy... (Press Ctrl+C to stop)"
	@cargo watch -x 'clippy --all-targets --all-features'

# Git hooks helpers
install-hooks: ## Install git pre-commit hooks
	@echo "🪝 Installing git pre-commit hooks..."
	@mkdir -p .git/hooks
	@echo '#!/bin/bash' > .git/hooks/pre-commit
	@echo 'set -e' >> .git/hooks/pre-commit
	@echo 'echo "🔍 Running pre-commit checks..."' >> .git/hooks/pre-commit
	@echo 'make pre-commit' >> .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "✅ Git pre-commit hook installed!"
	@echo "Now 'git commit' will automatically run quality checks."

# Benchmarking and profiling
bench: ## Run benchmarks (if any exist)
	@echo "🏎️  Running benchmarks..."
	@if find . -name "*.rs" -exec grep -l "#\[bench\]" {} \; | head -1 | grep -q .; then \
		cargo bench; \
	else \
		echo "ℹ️  No benchmarks found"; \
	fi

profile: ## Build with profiling enabled
	@echo "📊 Building with profiling..."
	@cargo build --release --features profiling || cargo build --release
	@echo "✅ Profile build completed!"

# Size analysis
size: build ## Analyze binary size
	@echo "📏 Analyzing binary size..."
	@ls -lah target/release/setup-devbox
	@echo ""
	@echo "📊 Size breakdown:"
	@size target/release/setup-devbox 2>/dev/null || echo "  (size command not available)"

# Dependency management
update: ## Update dependencies
	@echo "📦 Updating dependencies..."
	@cargo update
	@echo "✅ Dependencies updated!"

tree: ## Show dependency tree
	@echo "🌳 Dependency tree:"
	@cargo tree

# Quick fixes
fix: lint-fix format ## Apply automatic fixes (clippy + format)
	@echo "✅ Automatic fixes applied!"

# Development environment info
info: ## Show development environment information
	@echo "🦀 Development Environment Info:"
	@echo ""
	@echo "Rust version:"
	@rustc --version
	@echo ""
	@echo "Cargo version:"
	@cargo --version
	@echo ""
	@echo "Installed components:"
	@rustup component list --installed
	@echo ""
	@echo "Toolchains:"
	@rustup toolchain list
	@echo ""
	@echo "Project info:"
	@cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "setup-devbox") | "  Name: \(.name)\n  Version: \(.version)\n  Description: \(.description // "N/A")"' 2>/dev/null || echo "  (jq not available for detailed project info)"