#!/bin/bash

set -e

BASE_URL="http://localhost:3000"

echo "======================================"
echo "Testing System Admin Implementation"
echo "======================================"
echo ""

echo "1. Create System Admin (via CLI - should be done before running this script)..."
echo "   Interactive mode: cargo run --bin chalkbyte-cli -- create-sysadmin"
echo "   Non-interactive:  cargo run --bin chalkbyte-cli -- create-sysadmin \\"
echo "                       --first-name System --last-name Administrator \\"
echo "                       --email sysadmin@test.com --password password123"
echo "   Or with justfile: just create-sysadmin-interactive"
echo "   Assuming system admin already exists..."
echo ""

echo "2. Login as System Admin..."
SYSADMIN_TOKEN=$(curl -s -X POST $BASE_URL/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "sysadmin@test.com",
    "password": "password123"
  }' | jq -r '.access_token')
echo "Token obtained: ${SYSADMIN_TOKEN:0:50}..."
echo ""

echo "3. Create School (by System Admin)..."
SCHOOL_RESPONSE=$(curl -s -X POST $BASE_URL/api/schools \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $SYSADMIN_TOKEN" \
  -d '{
    "name": "Test High School",
    "address": "456 Education Ave"
  }')
echo "$SCHOOL_RESPONSE" | jq .
SCHOOL_ID=$(echo "$SCHOOL_RESPONSE" | jq -r '.id')
echo ""

echo "4. List All Schools (by System Admin)..."
curl -s -X GET $BASE_URL/api/schools \
  -H "Authorization: Bearer $SYSADMIN_TOKEN" | jq .
echo ""

echo "5. Create School Admin (by System Admin)..."
ADMIN_RESPONSE=$(curl -s -X POST $BASE_URL/api/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $SYSADMIN_TOKEN" \
  -d "{
    \"first_name\": \"School\",
    \"last_name\": \"Admin\",
    \"email\": \"admin@test.com\",
    \"role\": \"admin\",
    \"school_id\": \"$SCHOOL_ID\"
  }")
echo "$ADMIN_RESPONSE" | jq .
echo ""

echo "6. Try to create duplicate school (should fail)..."
curl -s -X POST $BASE_URL/api/schools \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $SYSADMIN_TOKEN" \
  -d '{
    "name": "Test High School",
    "address": "Duplicate attempt"
  }' | jq .
echo ""

echo "7. List all users (as System Admin)..."
curl -s -X GET $BASE_URL/api/users \
  -H "Authorization: Bearer $SYSADMIN_TOKEN" | jq 'length'
echo ""

echo "======================================"
echo "All tests completed!"
echo "======================================"
