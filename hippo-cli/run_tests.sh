#!/bin/bash
# Test runner for hippo-cli

set -e

echo "🦛 Hippo CLI Test Suite"
echo "======================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from hippo-cli directory${NC}"
    exit 1
fi

# Function to run tests with a label
run_test_suite() {
    local name=$1
    local cmd=$2

    echo -e "${YELLOW}Running $name...${NC}"
    if eval "$cmd"; then
        echo -e "${GREEN}✓ $name passed${NC}"
        echo ""
    else
        echo -e "${RED}✗ $name failed${NC}"
        exit 1
    fi
}

# Parse arguments
SHOW_OUTPUT=false
RUN_IGNORED=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --nocapture|-v|--verbose)
            SHOW_OUTPUT=true
            shift
            ;;
        --ignored)
            RUN_IGNORED=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--nocapture|--verbose] [--ignored]"
            exit 1
            ;;
    esac
done

# Set output flag
if [ "$SHOW_OUTPUT" = true ]; then
    OUTPUT_FLAG="-- --nocapture"
else
    OUTPUT_FLAG=""
fi

# Run unit tests
run_test_suite "Unit Tests" "cargo test --package hippo-cli command_parsing_tests $OUTPUT_FLAG"

# Run integration tests
run_test_suite "Integration Tests" "cargo test --package hippo-cli integration_tests $OUTPUT_FLAG"

# Run watch tests
run_test_suite "Watch Tests" "cargo test --package hippo-cli watch_tests $OUTPUT_FLAG"

# Run error tests
run_test_suite "Error Handling Tests" "cargo test --package hippo-cli error_tests $OUTPUT_FLAG"

# Run workflow tests
run_test_suite "Workflow Tests" "cargo test --package hippo-cli workflow_tests $OUTPUT_FLAG"

# Run brain tests (without API key)
run_test_suite "Brain Tests (No API)" "cargo test --package hippo-cli brain_tests::test_brain_without_api_key $OUTPUT_FLAG"

# Run ignored tests if requested
if [ "$RUN_IGNORED" = true ]; then
    if [ -z "$ANTHROPIC_API_KEY" ]; then
        echo -e "${YELLOW}Warning: ANTHROPIC_API_KEY not set, skipping AI tests${NC}"
    else
        run_test_suite "AI Tests (with API)" "cargo test --package hippo-cli -- --ignored $OUTPUT_FLAG"
    fi
fi

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}All tests passed! 🦛${NC}"
echo -e "${GREEN}================================${NC}"
