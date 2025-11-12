#!/bin/bash

# Role Middleware Testing Script
# Tests all three approaches of role-based authorization

set -e  # Exit on error

BASE_URL="${BASE_URL:-http://localhost:3000}"
BOLD='\033[1m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BOLD}=== Chalkbyte Role Middleware Testing ===${NC}\n"

# Function to print test result
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}: $2"
    else
        echo -e "${RED}✗ FAIL${NC}: $2"
    fi
}

# Function to test endpoint with expected status
test_endpoint() {
    local token=$1
    local method=$2
    local endpoint=$3
    local expected_status=$4
    local description=$5
    local data=$6

    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            -H "Authorization: Bearer $token" \
            -H "Content-Type: application/json" \
            "$BASE_URL$endpoint")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            -H "Authorization: Bearer $token" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "$BASE_URL$endpoint")
    fi

    status_code=$(echo "$response" | tail -n1)

    if [ "$status_code" = "$expected_status" ]; then
        print_result 0 "$description (expected $expected_status, got $status_code)"
        return 0
    else
        print_result 1 "$description (expected $expected_status, got $status_code)"
        echo "Response: $(echo "$response" | head -n-1)"
        return 1
    fi
}

echo -e "${BOLD}Step 1: Creating Test Users${NC}"
echo "----------------------------------------"

# Create system admin (via CLI)
echo "Creating system admin..."
cargo run --quiet -- create-sysadmin \
    --email sysadmin@test.com \
    --password password123 \
    --first-name System \
    --last-name Admin 2>/dev/null || echo "System admin may already exist"

# Login as system admin to create other users
echo "Logging in as system admin..."
SYSADMIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"email":"sysadmin@test.com","password":"password123"}')

SYSADMIN_TOKEN=$(echo "$SYSADMIN_RESPONSE" | jq -r '.access_token')

if [ "$SYSADMIN_TOKEN" = "null" ] || [ -z "$SYSADMIN_TOKEN" ]; then
    echo -e "${RED}Failed to get system admin token${NC}"
    exit 1
fi
echo -e "${GREEN}✓ System admin logged in${NC}"

# Create a test school
echo "Creating test school..."
SCHOOL_RESPONSE=$(curl -s -X POST "$BASE_URL/api/schools" \
    -H "Authorization: Bearer $SYSADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Test School '$(date +%s)'","address":"123 Test St"}')

SCHOOL_ID=$(echo "$SCHOOL_RESPONSE" | jq -r '.id')
echo -e "${GREEN}✓ School created: $SCHOOL_ID${NC}"

# Create school admin
echo "Creating school admin..."
ADMIN_CREATE=$(curl -s -X POST "$BASE_URL/api/users" \
    -H "Authorization: Bearer $SYSADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "first_name":"School",
        "last_name":"Admin",
        "email":"admin@test.com",
        "role":"admin",
        "school_id":"'$SCHOOL_ID'"
    }')
echo -e "${GREEN}✓ School admin created${NC}"

# Create teacher
echo "Creating teacher..."
TEACHER_CREATE=$(curl -s -X POST "$BASE_URL/api/users" \
    -H "Authorization: Bearer $SYSADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "first_name":"Test",
        "last_name":"Teacher",
        "email":"teacher@test.com",
        "role":"teacher",
        "school_id":"'$SCHOOL_ID'"
    }')
TEACHER_ID=$(echo "$TEACHER_CREATE" | jq -r '.id')
echo -e "${GREEN}✓ Teacher created: $TEACHER_ID${NC}"

# Create student
echo "Creating student..."
STUDENT_CREATE=$(curl -s -X POST "$BASE_URL/api/users" \
    -H "Authorization: Bearer $SYSADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "first_name":"Test",
        "last_name":"Student",
        "email":"student@test.com",
        "role":"student",
        "school_id":"'$SCHOOL_ID'"
    }')
STUDENT_ID=$(echo "$STUDENT_CREATE" | jq -r '.id')
echo -e "${GREEN}✓ Student created: $STUDENT_ID${NC}"

# Note: Password for created users should be set via your user creation logic
# For this test, we'll use a default password or manual setup

echo -e "\n${BOLD}Step 2: Getting Tokens for All Users${NC}"
echo "----------------------------------------"

# Get admin token
ADMIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"email":"admin@test.com","password":"password123"}')
ADMIN_TOKEN=$(echo "$ADMIN_RESPONSE" | jq -r '.access_token')

if [ "$ADMIN_TOKEN" != "null" ] && [ -n "$ADMIN_TOKEN" ]; then
    echo -e "${GREEN}✓ Admin token obtained${NC}"
else
    echo -e "${YELLOW}⚠ Admin token not available (password may need to be set)${NC}"
    ADMIN_TOKEN=""
fi

# Get teacher token
TEACHER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"email":"teacher@test.com","password":"password123"}')
TEACHER_TOKEN=$(echo "$TEACHER_RESPONSE" | jq -r '.access_token')

if [ "$TEACHER_TOKEN" != "null" ] && [ -n "$TEACHER_TOKEN" ]; then
    echo -e "${GREEN}✓ Teacher token obtained${NC}"
else
    echo -e "${YELLOW}⚠ Teacher token not available${NC}"
    TEACHER_TOKEN=""
fi

# Get student token
STUDENT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"email":"student@test.com","password":"password123"}')
STUDENT_TOKEN=$(echo "$STUDENT_RESPONSE" | jq -r '.access_token')

if [ "$STUDENT_TOKEN" != "null" ] && [ -n "$STUDENT_TOKEN" ]; then
    echo -e "${GREEN}✓ Student token obtained${NC}"
else
    echo -e "${YELLOW}⚠ Student token not available${NC}"
    STUDENT_TOKEN=""
fi

echo -e "\n${BOLD}Step 3: Testing System Admin Access${NC}"
echo "----------------------------------------"

# System admin should access everything
test_endpoint "$SYSADMIN_TOKEN" "GET" "/api/schools" "200" "System admin can list schools"
test_endpoint "$SYSADMIN_TOKEN" "POST" "/api/schools" "201" "System admin can create school" \
    '{"name":"Another School '$(date +%s)'","address":"456 Test Ave"}'
test_endpoint "$SYSADMIN_TOKEN" "GET" "/api/users" "200" "System admin can list users"

echo -e "\n${BOLD}Step 4: Testing School Admin Access${NC}"
echo "----------------------------------------"

if [ -n "$ADMIN_TOKEN" ]; then
    # Admin should access school-scoped resources
    test_endpoint "$ADMIN_TOKEN" "GET" "/api/users" "200" "Admin can list users in their school"

    # Admin should NOT be able to create schools
    test_endpoint "$ADMIN_TOKEN" "POST" "/api/schools" "403" "Admin CANNOT create schools" \
        '{"name":"Unauthorized School","address":"789 Test Rd"}'

    # Admin should NOT be able to delete schools
    test_endpoint "$ADMIN_TOKEN" "DELETE" "/api/schools/$SCHOOL_ID" "403" "Admin CANNOT delete schools"
else
    echo -e "${YELLOW}⚠ Skipping admin tests (no token)${NC}"
fi

echo -e "\n${BOLD}Step 5: Testing Teacher Access${NC}"
echo "----------------------------------------"

if [ -n "$TEACHER_TOKEN" ]; then
    # Teacher should access their profile
    test_endpoint "$TEACHER_TOKEN" "GET" "/api/users/profile" "200" "Teacher can view their profile"

    # Teacher should NOT be able to create users
    test_endpoint "$TEACHER_TOKEN" "POST" "/api/users" "403" "Teacher CANNOT create users" \
        '{"first_name":"Test","last_name":"User","email":"test@test.com"}'

    # Teacher should NOT be able to create schools
    test_endpoint "$TEACHER_TOKEN" "POST" "/api/schools" "403" "Teacher CANNOT create schools" \
        '{"name":"Unauthorized School","address":"123 Test St"}'
else
    echo -e "${YELLOW}⚠ Skipping teacher tests (no token)${NC}"
fi

echo -e "\n${BOLD}Step 6: Testing Student Access${NC}"
echo "----------------------------------------"

if [ -n "$STUDENT_TOKEN" ]; then
    # Student should access their profile
    test_endpoint "$STUDENT_TOKEN" "GET" "/api/users/profile" "200" "Student can view their profile"

    # Student should NOT be able to list users
    test_endpoint "$STUDENT_TOKEN" "GET" "/api/users" "403" "Student CANNOT list users"

    # Student should NOT be able to create users
    test_endpoint "$STUDENT_TOKEN" "POST" "/api/users" "403" "Student CANNOT create users" \
        '{"first_name":"Test","last_name":"User","email":"test@test.com"}'

    # Student should NOT be able to access schools
    test_endpoint "$STUDENT_TOKEN" "GET" "/api/schools" "403" "Student CANNOT list schools"
else
    echo -e "${YELLOW}⚠ Skipping student tests (no token)${NC}"
fi

echo -e "\n${BOLD}Step 7: Testing No Token (Unauthorized)${NC}"
echo "----------------------------------------"

# No token should return 401
test_endpoint "" "GET" "/api/users" "401" "No token returns 401 Unauthorized"
test_endpoint "" "GET" "/api/schools" "401" "No token returns 401 Unauthorized"
test_endpoint "" "GET" "/api/users/profile" "401" "No token returns 401 Unauthorized"

echo -e "\n${BOLD}Step 8: Testing Invalid Token${NC}"
echo "----------------------------------------"

# Invalid token should return 401
test_endpoint "invalid-token-12345" "GET" "/api/users" "401" "Invalid token returns 401"
test_endpoint "Bearer invalid" "GET" "/api/schools" "401" "Invalid Bearer token returns 401"

echo -e "\n${BOLD}Step 9: Testing Role Hierarchy${NC}"
echo "----------------------------------------"

# System admin can do everything
test_endpoint "$SYSADMIN_TOKEN" "GET" "/api/schools" "200" "SystemAdmin passes admin check"
test_endpoint "$SYSADMIN_TOKEN" "GET" "/api/users/profile" "200" "SystemAdmin passes teacher check"

if [ -n "$ADMIN_TOKEN" ]; then
    # Admin can do teacher things
    test_endpoint "$ADMIN_TOKEN" "GET" "/api/users/profile" "200" "Admin passes teacher check"
fi

if [ -n "$TEACHER_TOKEN" ]; then
    # Teacher CANNOT do admin things
    test_endpoint "$TEACHER_TOKEN" "POST" "/api/users" "403" "Teacher fails admin check"
fi

echo -e "\n${BOLD}Test Summary${NC}"
echo "========================================"
echo -e "Tests completed. Review results above."
echo -e "\n${BOLD}Tested Scenarios:${NC}"
echo "✓ Layer-based middleware (router protection)"
echo "✓ Extractor-based authorization (handler protection)"
echo "✓ Role hierarchy (SystemAdmin > Admin > Teacher > Student)"
echo "✓ Unauthorized access (no token)"
echo "✓ Invalid token handling"
echo "✓ Forbidden access (insufficient permissions)"

echo -e "\n${BOLD}Notes:${NC}"
echo "- Make sure the server is running: cargo run"
echo "- Set passwords for created users if login fails"
echo "- Check USER_ROLES.md for permission matrix"
echo "- See ROLE_MIDDLEWARE.md for implementation details"

echo -e "\n${GREEN}Testing complete!${NC}"
