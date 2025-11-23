# API Usage Examples

Assuming the server is running on `http://127.0.0.1:8080`.

## 1. Create Account
Create a new account for Alice.
```bash
curl -X POST http://127.0.0.1:8080/accounts \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice"}'
```
*Response:* `{"id":<random_id>,"name":"Alice","balance":0}`

Create a new account for Bob.
```bash
curl -X POST http://127.0.0.1:8080/accounts \
  -H "Content-Type: application/json" \
  -d '{"name": "Bob"}'
```

## 2. Get Account
Get details for account with ID 1 (replace `1` with actual ID from creation).
```bash
curl http://127.0.0.1:8080/accounts/1
```

## 3. Deposit
Deposit 100 units into account 1.
```bash
curl -X POST http://127.0.0.1:8080/accounts/1/deposit \
  -H "Content-Type: application/json" \
  -d '{"amount": 100}'
```
*Response:* `{"id":1,"name":"Alice","balance":100}`

## 4. Withdraw
Withdraw 50 units from account 1.
```bash
curl -X POST http://127.0.0.1:8080/accounts/1/withdraw \
  -H "Content-Type: application/json" \
  -d '{"amount": 50}'
```
*Response:* `{"id":1,"name":"Alice","balance":50}`

## 5. Transfer
Transfer 25 units from account 1 to account 2.
```bash
curl -X POST http://127.0.0.1:8080/transfer \
  -H "Content-Type: application/json" \
  -d '{"from_account_id": 1, "to_account_id": 2, "amount": 25}'
```
*Response:* `200 OK`
