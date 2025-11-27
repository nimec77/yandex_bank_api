# API Usage Examples

Assuming the server is running on `http://127.0.0.1:8080`.

## Authentication Flow

All protected routes require a JWT token in the `Authorization: Bearer <token>` header. First, register a user, then login to get a token.

### 1. Health Check (Public)
Check if the server is running.
```bash
curl http://127.0.0.1:8080/api/health
```
*Response:* `{"status":"ok","timestamp":"2024-01-01T12:00:00Z"}`

### 2. Register User (Public)
Register a new user with email and password.
```bash
curl -X POST http://127.0.0.1:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com", "password": "secure123"}'
```
*Response:* `{"id":"<uuid>","email":"alice@example.com"}`

Register another user for Bob.
```bash
curl -X POST http://127.0.0.1:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "bob@example.com", "password": "secure456"}'
```
*Response:* `{"id":"<uuid>","email":"bob@example.com"}`

### 3. Login (Public)
Login to get a JWT access token.
```bash
curl -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com", "password": "secure123"}'
```
*Response:* `{"access_token":"eyJhbGc..."}`

Save the token for subsequent requests:
```bash
TOKEN=$(curl -s -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com", "password": "secure123"}' | jq -r '.access_token')
echo "Token: $TOKEN"
```

### 4. Get Token (Public)
Get a new token for an existing user by user ID.
```bash
curl -X POST http://127.0.0.1:8080/api/auth/token \
  -H "Content-Type: application/json" \
  -d '{"user_id": "<user-uuid>"}'
```
*Response:* `{"access_token":"eyJhbGc..."}`

## Account Operations (Protected - Require JWT)

All account operations require authentication. Use the token from login in the `Authorization` header.

### 5. Create Account
Create a new bank account for Alice.
```bash
curl -X POST http://127.0.0.1:8080/api/accounts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "Alice"}'
```
*Response:* `{"id":<random_id>,"name":"Alice","balance":0}`

Create an account for Bob (using Bob's token):
```bash
BOB_TOKEN=$(curl -s -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "bob@example.com", "password": "secure456"}' | jq -r '.access_token')

curl -X POST http://127.0.0.1:8080/api/accounts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $BOB_TOKEN" \
  -d '{"name": "Bob"}'
```
*Response:* `{"id":<random_id>,"name":"Bob","balance":0}`

### 6. Get Account
Get details for account with ID 1 (replace `1` with actual ID from creation).
```bash
curl http://127.0.0.1:8080/api/accounts/1 \
  -H "Authorization: Bearer $TOKEN"
```
*Response:* `{"id":1,"name":"Alice","balance":0}`

### 7. Deposit
Deposit 100 units into account 1.
```bash
curl -X POST http://127.0.0.1:8080/api/accounts/1/deposit \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"amount": 100}'
```
*Response:* `{"id":1,"name":"Alice","balance":100}`

### 8. Withdraw
Withdraw 50 units from account 1.
```bash
curl -X POST http://127.0.0.1:8080/api/accounts/1/withdraw \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"amount": 50}'
```
*Response:* `{"id":1,"name":"Alice","balance":50}`

### 9. Transfer
Transfer 25 units from account 1 to account 2.
```bash
curl -X POST http://127.0.0.1:8080/api/transfers \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"from_account_id": 1, "to_account_id": 2, "amount": 25}'
```
*Response:* `200 OK`

## Error Examples

### Unauthorized Access (Missing Token)
Attempting to access a protected route without a token:
```bash
curl http://127.0.0.1:8080/api/accounts/1
```
*Response:* `401 Unauthorized` with `{"error":"missing bearer","details":{"message":"missing bearer"}}`

### Invalid Token
Using an invalid or expired token:
```bash
curl http://127.0.0.1:8080/api/accounts/1 \
  -H "Authorization: Bearer invalid_token"
```
*Response:* `401 Unauthorized` with `{"error":"invalid token","details":{"message":"invalid token"}}`

### Invalid Credentials
Attempting to login with wrong password:
```bash
curl -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com", "password": "wrongpassword"}'
```
*Response:* `401 Unauthorized` with error message

## Complete Workflow Example

Here's a complete example showing the full flow:

```bash
# 1. Register a user
REGISTER_RESPONSE=$(curl -s -X POST http://127.0.0.1:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "secure123"}')
echo "Registration: $REGISTER_RESPONSE"

# 2. Login to get token
LOGIN_RESPONSE=$(curl -s -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "secure123"}')
TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.access_token')
echo "Token: $TOKEN"

# 3. Create an account
ACCOUNT_RESPONSE=$(curl -s -X POST http://127.0.0.1:8080/api/accounts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "My Account"}')
ACCOUNT_ID=$(echo $ACCOUNT_RESPONSE | jq -r '.id')
echo "Account created: $ACCOUNT_RESPONSE"

# 4. Check balance
curl -s http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID \
  -H "Authorization: Bearer $TOKEN" | jq

# 5. Deposit money
curl -s -X POST http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID/deposit \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"amount": 1000}' | jq

# 6. Check balance again
curl -s http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID \
  -H "Authorization: Bearer $TOKEN" | jq
```

## CORS Testing

### Allowed Origin
Request from an allowed origin (configured in `ALLOWED_ORIGINS`):
```bash
curl -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: POST" \
  -X OPTIONS http://127.0.0.1:8080/api/accounts
```

### Disallowed Origin
Request from an unallowed origin (should be blocked):
```bash
curl -H "Origin: https://evil.com" \
  -X POST http://127.0.0.1:8080/api/accounts \
  -H "Content-Type: application/json" \
  -d '{"name": "Test"}'
```
