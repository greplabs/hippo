#!/bin/bash
# CI/CD Setup Verification Script
# Checks if all required tools and configurations are in place

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0
WARNINGS=0

# Helper functions
print_header() {
    echo -e "\n${BLUE}===================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}===================================${NC}\n"
}

check_pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((PASSED++))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ((FAILED++))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((WARNINGS++))
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Main checks
print_header "Hippo CI/CD Setup Verification"

# 1. Git checks
print_header "1. Git Configuration"

if command_exists git; then
    check_pass "Git is installed"

    if git rev-parse --git-dir > /dev/null 2>&1; then
        check_pass "Repository is a git repository"

        if git remote -v | grep -q origin; then
            check_pass "Git remote 'origin' is configured"
        else
            check_warn "No git remote 'origin' found"
        fi
    else
        check_fail "Not in a git repository"
    fi
else
    check_fail "Git is not installed"
fi

# 2. Rust toolchain
print_header "2. Rust Toolchain"

if command_exists rustc; then
    VERSION=$(rustc --version)
    check_pass "Rust is installed ($VERSION)"

    if command_exists cargo; then
        check_pass "Cargo is installed"
    else
        check_fail "Cargo is not installed"
    fi

    # Check for required targets
    if rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
        check_pass "Linux target installed"
    else
        check_warn "Linux target not installed (rustup target add x86_64-unknown-linux-gnu)"
    fi
else
    check_fail "Rust is not installed"
fi

# 3. Required tools
print_header "3. Required Tools"

if command_exists rustfmt; then
    check_pass "rustfmt is installed"
else
    check_fail "rustfmt not found (rustup component add rustfmt)"
fi

if command_exists cargo-clippy; then
    check_pass "clippy is installed"
else
    check_fail "clippy not found (rustup component add clippy)"
fi

# 4. Optional tools
print_header "4. Optional Tools (Enhanced CI)"

if command_exists gh; then
    check_pass "GitHub CLI (gh) is installed"
else
    check_warn "GitHub CLI not found (install for easier workflow management)"
fi

if command_exists act; then
    check_pass "act (local GitHub Actions) is installed"
else
    check_warn "act not found (install for local workflow testing)"
fi

if command_exists cargo-tarpaulin; then
    check_pass "cargo-tarpaulin is installed"
else
    check_warn "cargo-tarpaulin not found (install for coverage reports)"
fi

if command_exists cargo-audit; then
    check_pass "cargo-audit is installed"
else
    check_warn "cargo-audit not found (install for security audits)"
fi

if command_exists cargo-deny; then
    check_pass "cargo-deny is installed"
else
    check_warn "cargo-deny not found (install for dependency checks)"
fi

if command_exists docker; then
    check_pass "Docker is installed"
else
    check_warn "Docker not found (install for container builds)"
fi

# 5. Workflow files
print_header "5. Workflow Files"

WORKFLOWS_DIR=".github/workflows"
if [ -d "$WORKFLOWS_DIR" ]; then
    check_pass "Workflows directory exists"

    REQUIRED_WORKFLOWS=("ci.yml" "test.yml" "release.yml")
    for workflow in "${REQUIRED_WORKFLOWS[@]}"; do
        if [ -f "$WORKFLOWS_DIR/$workflow" ]; then
            check_pass "Workflow $workflow exists"
        else
            check_fail "Workflow $workflow is missing"
        fi
    done

    WORKFLOW_COUNT=$(find "$WORKFLOWS_DIR" -name "*.yml" -type f | wc -l)
    echo -e "  ${BLUE}ℹ${NC} Total workflows: $WORKFLOW_COUNT"
else
    check_fail "Workflows directory not found"
fi

# 6. Configuration files
print_header "6. Configuration Files"

CONFIG_FILES=("Cargo.toml" "deny.toml" "Dockerfile" ".dockerignore")
for config in "${CONFIG_FILES[@]}"; do
    if [ -f "$config" ]; then
        check_pass "$config exists"
    else
        if [ "$config" = "deny.toml" ] || [ "$config" = "Dockerfile" ] || [ "$config" = ".dockerignore" ]; then
            check_warn "$config not found (optional)"
        else
            check_fail "$config not found"
        fi
    fi
done

# 7. Platform dependencies (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    print_header "7. Linux Platform Dependencies"

    REQUIRED_LIBS=("libssl-dev" "libgtk-3-dev" "libwebkit2gtk-4.1-dev")
    for lib in "${REQUIRED_LIBS[@]}"; do
        if dpkg -l | grep -q "$lib"; then
            check_pass "$lib is installed"
        else
            check_warn "$lib not found (required for Tauri builds)"
        fi
    done
fi

# 8. Project structure
print_header "8. Project Structure"

if [ -d "hippo-core" ]; then
    check_pass "hippo-core directory exists"
else
    check_fail "hippo-core directory not found"
fi

if [ -d "hippo-cli" ]; then
    check_pass "hippo-cli directory exists"
else
    check_warn "hippo-cli directory not found"
fi

if [ -d "hippo-tauri" ]; then
    check_pass "hippo-tauri directory exists"
else
    check_warn "hippo-tauri directory not found"
fi

# 9. Documentation
print_header "9. Documentation"

DOC_FILES=(".github/WORKFLOWS.md" ".github/CI_SETUP.md" "README.md")
for doc in "${DOC_FILES[@]}"; do
    if [ -f "$doc" ]; then
        check_pass "$doc exists"
    else
        check_warn "$doc not found (recommended)"
    fi
done

# 10. GitHub Actions status (if gh is available)
if command_exists gh; then
    print_header "10. GitHub Actions Status"

    if gh auth status >/dev/null 2>&1; then
        check_pass "GitHub CLI is authenticated"

        # Check if Actions are enabled
        if gh api repos/:owner/:repo/actions/permissions >/dev/null 2>&1; then
            check_pass "GitHub Actions are accessible"

            # List recent workflow runs
            echo -e "\n  ${BLUE}Recent workflow runs:${NC}"
            gh run list --limit 5 2>/dev/null || echo "  No recent runs"
        else
            check_warn "Cannot access GitHub Actions (check repository settings)"
        fi
    else
        check_warn "GitHub CLI not authenticated (run 'gh auth login')"
    fi
fi

# 11. Secrets check (if gh is available)
if command_exists gh && gh auth status >/dev/null 2>&1; then
    print_header "11. Repository Secrets"

    OPTIONAL_SECRETS=("CODECOV_TOKEN" "ANTHROPIC_API_KEY" "CARGO_REGISTRY_TOKEN")
    for secret in "${OPTIONAL_SECRETS[@]}"; do
        if gh secret list | grep -q "$secret"; then
            check_pass "$secret is configured"
        else
            check_warn "$secret not configured (optional, enables enhanced features)"
        fi
    done
fi

# Summary
print_header "Summary"

TOTAL=$((PASSED + FAILED + WARNINGS))
echo -e "${GREEN}Passed:${NC}   $PASSED"
echo -e "${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "${RED}Failed:${NC}   $FAILED"
echo -e "${BLUE}Total:${NC}    $TOTAL"

echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ CI/CD setup is ready!${NC}"

    if [ $WARNINGS -gt 0 ]; then
        echo -e "${YELLOW}⚠ There are $WARNINGS warnings. Consider addressing them for full functionality.${NC}"
    fi

    echo ""
    echo "Next steps:"
    echo "  1. Push your code to GitHub"
    echo "  2. Check the Actions tab for workflow runs"
    echo "  3. Create a tag (v1.0.0) to trigger a release"
    echo ""
    echo "Documentation:"
    echo "  - Workflow docs: .github/WORKFLOWS.md"
    echo "  - Setup guide: .github/CI_SETUP.md"
    echo "  - Summary: .github/CICD_SUMMARY.md"
    exit 0
else
    echo -e "${RED}✗ CI/CD setup has issues. Please address the failures above.${NC}"
    echo ""
    echo "Common fixes:"
    echo "  - Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  - Install rustfmt: rustup component add rustfmt"
    echo "  - Install clippy: rustup component add clippy"
    echo "  - Install GitHub CLI: https://cli.github.com/"
    echo ""
    echo "For detailed setup instructions, see: .github/CI_SETUP.md"
    exit 1
fi
