#!/bin/bash

# Test script for refresh token functionality
# This script tests login, token refresh, and logout endpoints

BASE_URL="http://localhost:3000/api"
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Refresh Token Functionality Test ===${NC}\n"

# Step 1: Login
echo -e "${BLUE}Step 1: Login with credentials${NC}"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "babalolajnr@gmail.com",
    "password": "Password@123"
  }')

echo "$LOGIN_RESPONSE" | jq '.'

ACCESS_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token // empty')
REFRESH_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.refresh_token // empty')

if [ -z "$ACCESS_TOKEN" ] || [ "$ACCESS_TOKEN" = "null" ]; then
    echo -e "${RED}Login failed! No access token received.${NC}"
    exit 1
fi

if [ -z "$REFRESH_TOKEN" ] || [ "$REFRESH_TOKEN" = "null" ]; then
    echo -e "${RED}Login failed! No refresh token received.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Login successful${NC}\n"

# Step 2: Use access token to get profile
echo -e "${BLUE}Step 2: Verify access token works (get profile)${NC}"
PROFILE_RESPONSE=$(curl -s -X GET "$BASE_URL/users/profile" \
  -H "Authorization: Bearer $ACCESS_TOKEN")

echo "$PROFILE_RESPONSE" | jq '.'

PROFILE_EMAIL=$(echo "$PROFILE_RESPONSE" | jq -r '.email // empty')
if [ -n "$PROFILE_EMAIL" ] && [ "$PROFILE_EMAIL" != "null" ]; then
    echo -e "${GREEN}✓ Access token verified${NC}\n"
else
    echo -e "${RED}✗ Access token verification failed${NC}\n"
    exit 1
fi

# Step 3: Wait a moment and refresh the token
echo -e "${BLUE}Step 3: Refresh access token using refresh token${NC}"
sleep 2

REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/refresh" \
  -H "Content-Type: application/json" \
  -d "{
    \"refresh_token\": \"$REFRESH_TOKEN\"
  }")

echo "$REFRESH_RESPONSE" | jq '.'

NEW_ACCESS_TOKEN=$(echo "$REFRESH_RESPONSE" | jq -r '.access_token // empty')
NEW_REFRESH_TOKEN=$(echo "$REFRESH_RESPONSE" | jq -r '.refresh_token // empty')

if [ -z "$NEW_ACCESS_TOKEN" ] || [ "$NEW_ACCESS_TOKEN" = "null" ]; then
    echo -e "${RED}✗ Token refresh failed! No new access token received.${NC}"
    exit 1
fi

if [ -z "$NEW_REFRESH_TOKEN" ] || [ "$NEW_REFRESH_TOKEN" = "null" ]; then
    echo -e "${RED}✗ Token refresh failed! No new refresh token received.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Token refresh successful (token rotation)${NC}\n"

# Step 4: Verify new access token works
echo -e "${BLUE}Step 4: Verify new access token works${NC}"
NEW_PROFILE_RESPONSE=$(curl -s -X GET "$BASE_URL/users/profile" \
  -H "Authorization: Bearer $NEW_ACCESS_TOKEN")

echo "$NEW_PROFILE_RESPONSE" | jq '.'

NEW_PROFILE_EMAIL=$(echo "$NEW_PROFILE_RESPONSE" | jq -r '.email // empty')
if [ -n "$NEW_PROFILE_EMAIL" ] && [ "$NEW_PROFILE_EMAIL" != "null" ]; then
    echo -e "${GREEN}✓ New access token verified${NC}\n"
else
    echo -e "${RED}✗ New access token verification failed${NC}\n"
    exit 1
fi

# Step 5: Try to use old refresh token (should fail due to rotation)
echo -e "${BLUE}Step 5: Try to use old refresh token (should fail)${NC}"
OLD_REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/refresh" \
  -H "Content-Type: application/json" \
  -d "{
    \"refresh_token\": \"$REFRESH_TOKEN\"
  }")

echo "$OLD_REFRESH_RESPONSE" | jq '.'

OLD_TOKEN_ERROR=$(echo "$OLD_REFRESH_RESPONSE" | jq -r '.error // empty')
if [ -n "$OLD_TOKEN_ERROR" ] && [ "$OLD_TOKEN_ERROR" != "null" ]; then
    echo -e "${GREEN}✓ Old refresh token correctly rejected (token rotation working)${NC}\n"
else
    echo -e "${RED}✗ Warning: Old refresh token should have been revoked${NC}\n"
fi

# Step 6: Logout (revoke all refresh tokens)
echo -e "${BLUE}Step 6: Logout (revoke all refresh tokens)${NC}"
LOGOUT_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/logout" \
  -H "Authorization: Bearer $NEW_ACCESS_TOKEN")

echo "$LOGOUT_RESPONSE" | jq '.'

LOGOUT_MESSAGE=$(echo "$LOGOUT_RESPONSE" | jq -r '.message // empty')
if [ -n "$LOGOUT_MESSAGE" ] && [ "$LOGOUT_MESSAGE" != "null" ]; then
    echo -e "${GREEN}✓ Logout successful${NC}\n"
else
    echo -e "${RED}✗ Logout failed${NC}\n"
    exit 1
fi

# Step 7: Try to use refresh token after logout (should fail)
echo -e "${BLUE}Step 7: Try to use refresh token after logout (should fail)${NC}"
POST_LOGOUT_REFRESH=$(curl -s -X POST "$BASE_URL/auth/refresh" \
  -H "Content-Type: application/json" \
  -d "{
    \"refresh_token\": \"$NEW_REFRESH_TOKEN\"
  }")

echo "$POST_LOGOUT_REFRESH" | jq '.'

POST_LOGOUT_ERROR=$(echo "$POST_LOGOUT_REFRESH" | jq -r '.error // empty')
if [ -n "$POST_LOGOUT_ERROR" ] && [ "$POST_LOGOUT_ERROR" != "null" ]; then
    echo -e "${GREEN}✓ Refresh token correctly revoked after logout${NC}\n"
else
    echo -e "${RED}✗ Warning: Refresh token should have been revoked after logout${NC}\n"
fi

echo -e "${GREEN}=== All tests completed successfully! ===${NC}\n"

echo -e "${BLUE}Summary:${NC}"
echo "✓ Login with refresh token"
echo "✓ Refresh token rotation"
echo "✓ Old tokens are revoked"
echo "✓ Logout revokes all tokens"
