#!/bin/bash
set -e

# Make sure server is running before executing
echo "Checking if server is running on port 8080..."
if ! curl -s http://localhost:8080/dev/email-logs/latest > /dev/null && [ $? -ne 0 ]; then
  echo "Error: Server is not running on port 8080. Please run 'cargo run' in another terminal."
  exit 1
fi

echo "=== Step 1: Seeding Users ==="
SEED_RES=$(curl -s -X POST http://localhost:8080/seed/users)
echo "Seed Response: $SEED_RES"
ADMIN_ID=$(echo "$SEED_RES" | jq -r '.admin_id')
JB_ID=$(echo "$SEED_RES" | jq -r '.james_bond_id')
echo "Admin User ID: $ADMIN_ID"
echo "James Bond User ID: $JB_ID"
echo ""

echo "=== Step 2: Login as Admin ==="
LOGIN_RES=$(curl -s -X POST -H "Content-Type: application/json" -d '{"email":"admin@company.com","password":"AdminPassword123"}' http://localhost:8080/auth/login)
echo "Login Response: $LOGIN_RES"
CHALLENGE_ID=$(echo "$LOGIN_RES" | jq -r '.login_challenge_id')
echo "Challenge ID: $CHALLENGE_ID"
echo ""

echo "=== Step 3: Retrieve 2FA Code (Dev backdoor) ==="
EMAIL_LOG=$(curl -s http://localhost:8080/dev/email-logs/latest)
echo "Email Log: $EMAIL_LOG"
ADMIN_CODE=$(echo "$EMAIL_LOG" | jq -r '.code')
echo "Admin 2FA Code: $ADMIN_CODE"
echo ""

echo "=== Step 4: Verify Admin 2FA & Get JWT ==="
VERIFY_RES=$(curl -s -X POST -H "Content-Type: application/json" -d "{\"login_challenge_id\":\"$CHALLENGE_ID\",\"code\":\"$ADMIN_CODE\"}" http://localhost:8080/auth/verify-2fa)
echo "Verify Response: $VERIFY_RES"
ADMIN_TOKEN=$(echo "$VERIFY_RES" | jq -r '.token')
echo ""

echo "=== Step 5: Create Exactly 5 Tasks as Admin ==="
priorities=("high" "medium" "low" "medium" "high")
for i in {1..5}; do
  p=${priorities[$((i-1))]}
  echo "Creating Task $i (Priority: $p)..."
  curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json" -d "{\"title\":\"Task $i\",\"description\":\"Description $i\",\"priority\":\"$p\"}" http://localhost:8080/tasks
done
echo "Tasks created successfully."
echo ""

# Query task IDs directly from Postgres database for assignment
echo "Fetching created task IDs from PostgreSQL..."
TASK_IDS=($(psql -U postgres -d assessment_db -t -A -c "SELECT id FROM tasks ORDER BY created_at DESC LIMIT 5;"))
TASK_1=${TASK_IDS[0]}
TASK_2=${TASK_IDS[1]}
TASK_3=${TASK_IDS[2]}
echo "Selected task IDs to assign: $TASK_1, $TASK_2, $TASK_3"
echo ""

echo "=== Step 6: Assign Exactly 3 Tasks to James Bond ==="
ASSIGN_RES=$(curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json" -d "{\"user_id\":\"$JB_ID\",\"task_ids\":[\"$TASK_1\",\"$TASK_2\",\"$TASK_3\"]}" http://localhost:8080/tasks/assign)
echo "Assign Response: $ASSIGN_RES"
echo ""

echo "=== Step 7: Login as James Bond ==="
JB_LOGIN_RES=$(curl -s -X POST -H "Content-Type: application/json" -d '{"email":"jamesbond@example.com","password":"ShakenNotStirred"}' http://localhost:8080/auth/login)
echo "James Bond Login Response: $JB_LOGIN_RES"
JB_CHALLENGE_ID=$(echo "$JB_LOGIN_RES" | jq -r '.login_challenge_id')
echo "James Bond Challenge ID: $JB_CHALLENGE_ID"
echo ""

echo "=== Step 8: Retrieve James Bond 2FA Code ==="
JB_EMAIL_LOG=$(curl -s http://localhost:8080/dev/email-logs/latest)
echo "James Bond Email Log: $JB_EMAIL_LOG"
JB_CODE=$(echo "$JB_EMAIL_LOG" | jq -r '.code')
echo "James Bond 2FA Code: $JB_CODE"
echo ""

echo "=== Step 9: Verify James Bond 2FA & Get JWT ==="
JB_VERIFY_RES=$(curl -s -X POST -H "Content-Type: application/json" -d "{\"login_challenge_id\":\"$JB_CHALLENGE_ID\",\"code\":\"$JB_CODE\"}" http://localhost:8080/auth/verify-2fa)
echo "James Bond Verify Response: $JB_VERIFY_RES"
JB_TOKEN=$(echo "$JB_VERIFY_RES" | jq -r '.token')
echo ""

echo "=== Step 10: Attempt Task Creation as James Bond (Should fail with 403) ==="
curl -i -s -X POST -H "Authorization: Bearer $JB_TOKEN" -H "Content-Type: application/json" -d '{"title":"Illegal Task","priority":"medium"}' http://localhost:8080/tasks | head -n 1
echo ""

echo "=== Step 11: Call GET /tasks/view-my-tasks (1st Request - Cache Miss) ==="
curl -s -H "Authorization: Bearer $JB_TOKEN" http://localhost:8080/tasks/view-my-tasks | jq .
echo ""

echo "=== Step 12: Call GET /tasks/view-my-tasks (2nd Request - Cache Hit) ==="
curl -s -H "Authorization: Bearer $JB_TOKEN" http://localhost:8080/tasks/view-my-tasks | jq .
echo ""

echo "🎉 Workflow test completed successfully!"
