#!/bin/bash

BASE_URL="http://localhost:3000"

echo "=== Testing Profile Update and Password Change Endpoints ==="
echo

# Login as a user
echo "1. Logging in..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "student_school_1_1@example.com",
    "password": "password123"
  }')

TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.access_token')
USER_ID=$(echo $LOGIN_RESPONSE | jq -r '.user.id')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
    echo "❌ Login failed"
    echo $LOGIN_RESPONSE | jq
    exit 1
fi

echo "✅ Login successful"
echo "User ID: $USER_ID"
echo

# Get current profile
echo "2. Getting current profile..."
PROFILE=$(curl -s -X GET "$BASE_URL/api/users/profile" \
  -H "Authorization: Bearer $TOKEN")

echo $PROFILE | jq
echo

# Update profile (name only)
echo "3. Updating profile (changing first name and last name)..."
UPDATE_RESPONSE=$(curl -s -X PUT "$BASE_URL/api/users/profile" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "first_name": "UpdatedFirstName",
    "last_name": "UpdatedLastName"
  }')

echo $UPDATE_RESPONSE | jq
echo

# Get updated profile
echo "4. Getting updated profile..."
UPDATED_PROFILE=$(curl -s -X GET "$BASE_URL/api/users/profile" \
  -H "Authorization: Bearer $TOKEN")

echo $UPDATED_PROFILE | jq
echo

# Try to change password with wrong current password
echo "5. Testing password change with incorrect current password (should fail)..."
WRONG_PWD_RESPONSE=$(curl -s -X POST "$BASE_URL/api/users/profile/change-password" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "wrongpassword",
    "new_password": "newPassword123"
  }')

echo $WRONG_PWD_RESPONSE | jq
echo

# Change password with correct current password
echo "6. Changing password with correct current password..."
CHANGE_PWD_RESPONSE=$(curl -s -X POST "$BASE_URL/api/users/profile/change-password" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "password123",
    "new_password": "newPassword123"
  }')

echo $CHANGE_PWD_RESPONSE | jq
echo

# Try to login with old password (should fail)
echo "7. Trying to login with old password (should fail)..."
OLD_PWD_LOGIN=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "student_school_1_1@example.com",
    "password": "password123"
  }')

echo $OLD_PWD_LOGIN | jq
echo

# Login with new password
echo "8. Logging in with new password..."
NEW_PWD_LOGIN=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "student_school_1_1@example.com",
    "password": "newPassword123"
  }')

NEW_TOKEN=$(echo $NEW_PWD_LOGIN | jq -r '.access_token')

if [ "$NEW_TOKEN" = "null" ] || [ -z "$NEW_TOKEN" ]; then
    echo "❌ Login with new password failed"
    echo $NEW_PWD_LOGIN | jq
else
    echo "✅ Login with new password successful"
    echo "New Token: ${NEW_TOKEN:0:50}..."
fi
echo

# Change password back to original
echo "9. Changing password back to original..."
RESTORE_PWD=$(curl -s -X POST "$BASE_URL/api/users/profile/change-password" \
  -H "Authorization: Bearer $NEW_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "newPassword123",
    "new_password": "password123"
  }')

echo $RESTORE_PWD | jq
echo

# Restore original name
echo "10. Restoring original name..."
RESTORE_NAME=$(curl -s -X PUT "$BASE_URL/api/users/profile" \
  -H "Authorization: Bearer $NEW_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "first_name": "Student",
    "last_name": "School 1 1"
  }')

echo $RESTORE_NAME | jq
echo

echo "=== Test Complete ==="
