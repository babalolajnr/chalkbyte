#!/bin/bash

# Test Password Reset Flow
# Make sure the server is running and Mailpit is accessible

set -e

API_URL="http://localhost:3000/api"
MAILPIT_URL="http://localhost:8025"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Password Reset Flow Test ===${NC}\n"

# Get test email from argument or use default
TEST_EMAIL="${1:-test@example.com}"

echo -e "${YELLOW}Step 1: Request password reset for ${TEST_EMAIL}${NC}"
RESPONSE=$(curl -s -X POST "${API_URL}/auth/forgot-password" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${TEST_EMAIL}\"}")

echo "Response: $RESPONSE"
echo ""

echo -e "${YELLOW}Step 2: Check Mailpit for the email${NC}"
echo "Opening Mailpit in your browser..."
echo "URL: ${MAILPIT_URL}"
echo ""
echo "Please:"
echo "1. Go to ${MAILPIT_URL}"
echo "2. Find the password reset email"
echo "3. Copy the token from the URL"
echo ""

read -p "Enter the token from the email: " TOKEN

if [ -z "$TOKEN" ]; then
  echo -e "${RED}No token provided. Exiting.${NC}"
  exit 1
fi

echo ""
echo -e "${YELLOW}Step 3: Reset password with token${NC}"
NEW_PASSWORD="newPassword123"

RESPONSE=$(curl -s -X POST "${API_URL}/auth/reset-password" \
  -H "Content-Type: application/json" \
  -d "{\"token\":\"${TOKEN}\",\"new_password\":\"${NEW_PASSWORD}\"}")

echo "Response: $RESPONSE"
echo ""

if echo "$RESPONSE" | grep -q "successfully"; then
  echo -e "${GREEN}✓ Password reset successful!${NC}"
  echo ""
  
  echo -e "${YELLOW}Step 4: Test login with new password${NC}"
  LOGIN_RESPONSE=$(curl -s -X POST "${API_URL}/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"${TEST_EMAIL}\",\"password\":\"${NEW_PASSWORD}\"}")
  
  if echo "$LOGIN_RESPONSE" | grep -q "access_token"; then
    echo -e "${GREEN}✓ Login successful with new password!${NC}"
    echo "$LOGIN_RESPONSE" | jq '.' 2>/dev/null || echo "$LOGIN_RESPONSE"
  else
    echo -e "${RED}✗ Login failed${NC}"
    echo "$LOGIN_RESPONSE"
  fi
else
  echo -e "${RED}✗ Password reset failed${NC}"
fi

echo ""
echo -e "${GREEN}=== Test Complete ===${NC}"
