# Yandex Bank API

A production-ready REST API banking system built with Rust and Actix-web, demonstrating best practices in secure web development, clean architecture, and concurrent programming.

## Overview

This project implements a complete banking API with user authentication, account management, and financial operations. It showcases:

- **JWT-based authentication** with secure password hashing (Argon2id)
- **Clean architecture** with clear separation of concerns
- **Thread-safe concurrent access** using Arc and RwLock
- **Comprehensive error handling** and structured logging
- **Extensive test coverage** with unit and integration tests
- **Security-first design** with middleware protection

## Features

### Authentication & Authorization
- User registration with email and password
- Secure password hashing using Argon2id algorithm
- JWT token generation and validation (HS256)
- Token-based authentication middleware
- 1-hour token expiration with automatic validation

### Account Management
- Create bank accounts with custom names
- View account details and balance
- Deposit funds into accounts
- Withdraw funds (with balance validation)
- Transfer money between accounts

### Security & Middleware
- JWT authentication for protected routes
- Request ID generation for tracking
- Response timing headers for performance monitoring
- CORS support with configurable origins
- Security headers (X-Content-Type-Options, Referrer-Policy, etc.)

## Architecture

The project follows clean architecture principles with layered design:

```
src/
├── domain/              # Business entities and rules
│   ├── models.rs        # Core entities (Account, Amount)
│   ├── user.rs          # User entities and DTOs
│   ├── error.rs         # Domain error types
│   └── repository.rs    # Repository trait definitions
├── application/         # Application services
│   ├── service.rs       # Banking operations logic
│   └── auth_service.rs  # Authentication logic
├── presentation/        # HTTP layer
│   ├── handlers.rs      # API endpoint handlers
│   ├── auth.rs          # Auth route handlers
│   └── middleware.rs    # JWT, timing, request ID middleware
├── data/                # Data access layer
│   ├── memory.rs        # In-memory account storage
│   └── user_repository.rs # In-memory user storage
└── infrastructure/      # Cross-cutting concerns
    ├── security.rs      # Password hashing & JWT
    └── logging.rs       # Structured logging setup
```

## Prerequisites

- Rust 1.70+ (Edition 2024)
- Cargo package manager
- (Optional) jq for parsing JSON in curl examples

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd yandex_bank_api
```

2. Configure environment variables:
```bash
# Copy the example .env file or create your own
cat > .env << EOF
RUST_LOG=actix_web=info,bank_api=debug
ALLOWED_ORIGINS=http://localhost:3000,https://myapp.com
JWT_SECRET=your-super-secret-key-change-in-production
PORT=8080
EOF
```

3. Build the project:
```bash
cargo build --release
```

## Running the Application

### Development mode
```bash
cargo run
```

### Production mode
```bash
cargo run --release
```

The server will start on `http://127.0.0.1:8080` (or the port specified in your `.env` file).

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test suite
cargo test --test api_tests
cargo test --test auth_integration_tests
```

## API Documentation

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check endpoint |
| POST | `/api/auth/register` | Register a new user |
| POST | `/api/auth/login` | Login and get JWT token |
| POST | `/api/auth/token` | Get token for user by ID |

### Protected Endpoints (Require JWT)

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/accounts` | Create a new account |
| GET | `/api/accounts/{id}` | Get account details |
| POST | `/api/accounts/{id}/deposit` | Deposit funds |
| POST | `/api/accounts/{id}/withdraw` | Withdraw funds |
| POST | `/api/transfers` | Transfer between accounts |

## Usage Examples

### Complete Workflow

Here's a complete example showing registration, login, and account operations:

```bash
# 1. Health check
curl http://127.0.0.1:8080/api/health

# 2. Register a user
curl -X POST http://127.0.0.1:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com", "password": "secure123"}'
# Response: {"id":"<uuid>","email":"alice@example.com"}

# 3. Login to get token
TOKEN=$(curl -s -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com", "password": "secure123"}' | jq -r '.access_token')
echo "Token: $TOKEN"

# 4. Create an account
ACCOUNT=$(curl -s -X POST http://127.0.0.1:8080/api/accounts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "My Savings"}')
echo "Account created: $ACCOUNT"

ACCOUNT_ID=$(echo $ACCOUNT | jq -r '.id')

# 5. Deposit money
curl -s -X POST http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID/deposit \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"amount": 1000}' | jq
# Response: {"id":<id>,"name":"My Savings","balance":1000}

# 6. Check balance
curl -s http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID \
  -H "Authorization: Bearer $TOKEN" | jq
# Response: {"id":<id>,"name":"My Savings","balance":1000}

# 7. Withdraw money
curl -s -X POST http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID/withdraw \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"amount": 300}' | jq
# Response: {"id":<id>,"name":"My Savings","balance":700}
```

### Transfer Between Accounts

```bash
# Create second account for Bob
curl -X POST http://127.0.0.1:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "bob@example.com", "password": "secure456"}'

BOB_TOKEN=$(curl -s -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "bob@example.com", "password": "secure456"}' | jq -r '.access_token')

BOB_ACCOUNT=$(curl -s -X POST http://127.0.0.1:8080/api/accounts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $BOB_TOKEN" \
  -d '{"name": "Bob Account"}')
BOB_ACCOUNT_ID=$(echo $BOB_ACCOUNT | jq -r '.id')

# Transfer from Alice to Bob
curl -X POST http://127.0.0.1:8080/api/transfers \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d "{\"from_account_id\": $ACCOUNT_ID, \"to_account_id\": $BOB_ACCOUNT_ID, \"amount\": 200}"
# Response: 200 OK

# Check both balances
curl -s http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID \
  -H "Authorization: Bearer $TOKEN" | jq
# Alice's balance: 500

curl -s http://127.0.0.1:8080/api/accounts/$BOB_ACCOUNT_ID \
  -H "Authorization: Bearer $BOB_TOKEN" | jq
# Bob's balance: 200
```

### Error Handling Examples

```bash
# Unauthorized access (missing token)
curl http://127.0.0.1:8080/api/accounts/1
# Response: 401 Unauthorized - {"error":"missing bearer","details":{"message":"missing bearer"}}

# Invalid credentials
curl -X POST http://127.0.0.1:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "alice@example.com", "password": "wrongpassword"}'
# Response: 401 Unauthorized

# Insufficient funds
curl -X POST http://127.0.0.1:8080/api/accounts/$ACCOUNT_ID/withdraw \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"amount": 999999}'
# Response: 400 Bad Request - {"error":"insufficient funds"}

# Account not found
curl http://127.0.0.1:8080/api/accounts/999999 \
  -H "Authorization: Bearer $TOKEN"
# Response: 404 Not Found
```

## Configuration

Configuration is managed through environment variables in the `.env` file:

```env
# Logging configuration (trace, debug, info, warn, error)
RUST_LOG=actix_web=info,bank_api=debug

# CORS allowed origins (comma-separated)
ALLOWED_ORIGINS=http://localhost:3000,https://myapp.com

# JWT secret key (CHANGE THIS in production!)
JWT_SECRET=your-super-secret-key-change-in-production

# Server port
PORT=8080
```

## Security Features

### Password Security
- **Algorithm**: Argon2id (memory-hard, GPU-resistant)
- **Parameters**: 19MB memory cost, 2 iterations, 1 parallelism
- **Salt**: Cryptographically random per password

### JWT Configuration
- **Algorithm**: HS256 (HMAC with SHA-256)
- **Expiration**: 1 hour
- **Validation Leeway**: 60 seconds
- **Claims**: `sub` (user_id), `exp` (expiration), `iat` (issued at)

### HTTP Security Headers
- `X-Content-Type-Options: nosniff`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: geolocation=(), microphone=(), camera=()`
- `Cross-Origin-Opener-Policy: same-origin`

## Tech Stack

| Dependency | Version | Purpose |
|------------|---------|---------|
| actix-web | 4 | Web framework |
| tokio | 1 | Async runtime |
| serde/serde_json | 1.0 | Serialization |
| jsonwebtoken | 9 | JWT handling |
| argon2 | 0.5 | Password hashing |
| uuid | 1.0 | Unique ID generation |
| chrono | 0.4 | Date/time handling |
| anyhow | 1.0 | Error handling |
| thiserror | 2.0 | Error types |
| tracing | 0.1 | Structured logging |
| actix-cors | 0.7 | CORS middleware |

## Data Storage

Currently uses **in-memory storage** with thread-safe concurrent access:

- Accounts stored in `HashMap<u32, Account>` wrapped in `Arc<RwLock<>>`
- Users stored in `HashMap<String, User>` wrapped in `Arc<RwLock<>>`
- Data is lost on server restart (suitable for development/testing)

For production use, implement the repository traits with a persistent database (PostgreSQL, MongoDB, etc.).

## Testing

The project includes comprehensive test coverage:

- **60+ unit tests** across domain, application, and infrastructure layers
- **15+ integration tests** for API endpoints and authentication flows
- **Edge case testing** for concurrent access, validation, and error handling

Test categories:
- Password hashing and verification
- JWT token generation and validation
- Account operations (create, deposit, withdraw, transfer)
- Authentication flows (register, login, token validation)
- Error handling and validation
- Concurrent access patterns

## Development

### Code Quality
- Follows Rust best practices and idioms
- Clean architecture with clear layer separation
- Comprehensive error handling with custom error types
- Structured logging with `tracing` for observability
- Type-safe amount handling with wrapper types

### Adding New Features

1. **Domain Layer**: Define entities and business rules in `src/domain/`
2. **Application Layer**: Implement business logic in services
3. **Presentation Layer**: Create HTTP handlers in `src/presentation/`
4. **Data Layer**: Implement repository for data access
5. **Add Tests**: Write unit and integration tests

## Troubleshooting

### Token Expiration
Tokens expire after 1 hour. If you get a 401 error, login again to get a fresh token.

### Port Already in Use
If port 8080 is in use, change the `PORT` in your `.env` file.

### CORS Issues
Add your frontend origin to `ALLOWED_ORIGINS` in the `.env` file.

## License

This project is for educational and demonstration purposes.

## Contributing

Contributions are welcome! Please ensure:
- All tests pass (`cargo test`)
- Code follows Rust conventions (`cargo fmt`, `cargo clippy`)
- New features include tests
- Documentation is updated

## More Examples

For additional curl examples and complete workflows, see [curl_examples.md](./curl_examples.md).

## Contact

For issues or questions, please open an issue on the repository.
