#!/usr/bin/env bash
# End-to-end validation of the 11-step workflow from the assignment brief.
#
#   ./scripts/validate.sh                       # API at http://localhost:8080
#   API_URL=http://localhost:3000/api ./scripts/validate.sh   # via the frontend's nginx proxy
#
# Requires: curl, jq. The script calls POST /dev/reset first so it can be run
# repeatedly and still create exactly 5 tasks / assign exactly 3.
set -euo pipefail

API_URL="${API_URL:-http://localhost:8080}"
ADMIN_EMAIL="admin@example.com"
ADMIN_PASSWORD="${SEED_ADMIN_PASSWORD:-Admin@12345}"
BOND_EMAIL="jamesbond@example.com"
BOND_PASSWORD="${SEED_STAFF_PASSWORD:-Bond@007007}"

command -v jq >/dev/null || { echo "jq is required (https://jqlang.github.io/jq/)"; exit 1; }

green() { printf '\033[32m%s\033[0m\n' "$*"; }
step()  { printf '\n\033[1;36m== Step %s\033[0m\n' "$*"; }
fail()  { printf '\033[31mFAIL: %s\033[0m\n' "$*"; exit 1; }

# call METHOD PATH [TOKEN] [JSON] -> sets $STATUS and $BODY
call() {
  local method=$1 path=$2 token=${3:-} data=${4:-}
  local args=(-sS -X "$method" -w '\n%{http_code}' -H 'Content-Type: application/json')
  [[ -n $token ]] && args+=(-H "Authorization: Bearer $token")
  [[ -n $data ]] && args+=(-d "$data")
  local out
  out=$(curl "${args[@]}" "$API_URL$path")
  STATUS=${out##*$'\n'}
  BODY=${out%$'\n'*}
}

expect() { [[ $STATUS == "$1" ]] || fail "expected HTTP $1, got $STATUS: $BODY"; }

login() { # login EMAIL PASSWORD -> echoes JWT
  local email=$1 password=$2
  call POST /auth/login "" "{\"email\":\"$email\",\"password\":\"$password\"}"
  expect 200
  local challenge; challenge=$(jq -r .login_challenge_id <<<"$BODY")
  [[ $(jq -r '.access_token // empty' <<<"$BODY") == "" ]] || fail "login returned a JWT before 2FA"
  echo "   login_challenge_id=$challenge (no JWT yet)" >&2

  call GET "/dev/email-logs/latest?email=$email"
  expect 200
  local code; code=$(jq -r .verification_code <<<"$BODY")
  echo "   code from dev email log: $code" >&2

  call POST /auth/verify-2fa "" "{\"login_challenge_id\":\"$challenge\",\"code\":\"$code\"}"
  expect 200
  jq -r .access_token <<<"$BODY"
}

echo "API: $API_URL"
call GET /health; expect 200; echo "health: $BODY"
call POST /dev/reset; expect 200; echo "reset:  $BODY"

step "1  Create Admin and James Bond"
call POST /seed/users; expect 200
jq -c '.users[] | {email, role, created}' <<<"$BODY"

step "2-4  Admin login -> 2FA code -> JWT"
ADMIN_TOKEN=$(login "$ADMIN_EMAIL" "$ADMIN_PASSWORD")
green "   Admin JWT: ${ADMIN_TOKEN:0:24}..."

step "5  Create exactly 5 tasks as Admin"
TASK_IDS=()
while IFS='|' read -r title priority; do
  call POST /tasks "$ADMIN_TOKEN" "{\"title\":\"$title\",\"description\":\"$title\",\"priority\":\"$priority\"}"
  expect 201
  TASK_IDS+=("$(jq -r .id <<<"$BODY")")
  echo "   created [$priority] $title"
done <<'EOF'
Infiltrate SPECTRE headquarters|high
Recover the Lektor decoder|medium
Debrief M on Operation Thunderball|low
Audit Q-Branch gadget inventory|medium
Renew Aston Martin DB5 insurance|low
EOF
[[ ${#TASK_IDS[@]} -eq 5 ]] || fail "expected 5 tasks"

step "6  Assign exactly 3 tasks to James Bond"
IDS_JSON=$(printf '%s\n' "${TASK_IDS[@]:0:3}" | jq -R . | jq -sc .)
call POST /tasks/assign "$ADMIN_TOKEN" "{\"task_ids\":$IDS_JSON,\"assignee_email\":\"$BOND_EMAIL\"}"
expect 200
echo "   assigned_count=$(jq .assigned_count <<<"$BODY")"

step "7-8  James Bond login -> 2FA code -> JWT"
BOND_TOKEN=$(login "$BOND_EMAIL" "$BOND_PASSWORD")
green "   James Bond JWT: ${BOND_TOKEN:0:24}..."

step "9  James Bond tries to create a task (expect 403)"
call POST /tasks "$BOND_TOKEN" '{"title":"Unauthorised mission"}'
expect 403
green "   403 Forbidden: $(jq -r .error.message <<<"$BODY")"

step "10  GET /tasks/view-my-tasks as James Bond (expect 3 tasks, cache.hit=false)"
call GET /tasks/view-my-tasks "$BOND_TOKEN"; expect 200
[[ $(jq '.tasks | length' <<<"$BODY") == 3 ]] || fail "expected 3 tasks"
[[ $(jq .cache.hit <<<"$BODY") == false ]] || fail "expected cache.hit=false on first call"
jq '{user, tasks: [.tasks[] | {id, title, status, priority, assigned_to}], summary, cache}' <<<"$BODY"

step "11  Call again (expect cache.hit=true)"
call GET /tasks/view-my-tasks "$BOND_TOKEN"; expect 200
[[ $(jq .cache.hit <<<"$BODY") == true ]] || fail "expected cache.hit=true on second call"
jq '{user, tasks: [.tasks[] | {id, title, status, priority, assigned_to}], summary, cache}' <<<"$BODY"

echo
green "All 11 validation steps passed."
