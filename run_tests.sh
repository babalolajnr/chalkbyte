#!/bin/bash

set -e

echo "🧪 Chalkbyte Test Suite"
echo "======================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if .env.test exists
if [ ! -f .env.test ]; then
    echo -e "${YELLOW}⚠️  .env.test not found. Using default test configuration.${NC}"
fi

# Load test environment
export $(cat .env.test 2>/dev/null | grep -v '^#' | xargs) || true

# Function to run tests
run_test_suite() {
    local name=$1
    local command=$2

    echo -e "${BLUE}Running $name...${NC}"
    if $command; then
        echo -e "${GREEN}✓ $name passed${NC}"
        echo ""
        return 0
    else
        echo -e "${YELLOW}✗ $name failed${NC}"
        echo ""
        return 1
    fi
}

# Parse arguments
UNIT_ONLY=false
INTEGRATION_ONLY=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --unit)
            UNIT_ONLY=true
            shift
            ;;
        --integration)
            INTEGRATION_ONLY=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: ./run_tests.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --unit          Run only unit tests"
            echo "  --integration   Run only integration tests"
            echo "  --verbose       Show detailed test output"
            echo "  --help          Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Determine test flags
TEST_FLAGS=""
if [ "$VERBOSE" = true ]; then
    TEST_FLAGS="-- --nocapture"
fi

FAILED=0

if [ "$INTEGRATION_ONLY" = false ]; then
    echo -e "${BLUE}📦 Unit Tests${NC}"
    echo "============="
    echo ""

    run_test_suite "Password Utilities (10 tests)" "cargo test --test unit_password $TEST_FLAGS" || FAILED=$((FAILED+1))
    run_test_suite "JWT Utilities (14 tests)" "cargo test --test unit_jwt $TEST_FLAGS" || FAILED=$((FAILED+1))
fi

if [ "$UNIT_ONLY" = false ]; then
    echo -e "${BLUE}🌐 Integration Tests${NC}"
    echo "===================="
    echo ""

    # Check if database is accessible
    if [ -z "$DATABASE_URL" ]; then
        echo -e "${YELLOW}⚠️  Warning: DATABASE_URL not set${NC}"
        echo "   Integration tests require a test database."
        echo "   Set DATABASE_URL in .env.test or environment"
        echo "   Example: DATABASE_URL=postgresql://postgres:postgres@localhost:5432/chalkbyte_test"
        echo ""
        FAILED=$((FAILED+1))
    elif ! psql "$DATABASE_URL" -c "SELECT 1;" > /dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  Warning: Cannot connect to test database${NC}"
        echo "   Database URL: $DATABASE_URL"
        echo "   Please ensure PostgreSQL is running and test database exists."
        echo "   Run: just test-db-setup"
        echo ""
        FAILED=$((FAILED+1))
    else
        run_test_suite "Authentication Tests (6 tests)" "cargo test --test integration_auth $TEST_FLAGS" || FAILED=$((FAILED+1))
    fi
fi

# Summary
echo "======================="
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${YELLOW}✗ $FAILED test suite(s) failed${NC}"
    exit 1
fi
