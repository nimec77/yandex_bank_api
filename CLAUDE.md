# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Commands

```bash
# Development
cargo run                           # Run server (requires .env with JWT_SECRET)
cargo build --release              # Production build

# Testing
cargo test                          # All tests
cargo test -- --nocapture          # Show test output
cargo test --test api_tests        # Specific integration test suite
cargo test <test_name>             # Run single test

# Code Quality
cargo fmt
cargo clippy
```

## Environment Setup

Required `.env` file in project root:
```env
JWT_SECRET=your-secret-key-here
RUST_LOG=actix_web=info,yandex_bank_api=debug
ALLOWED_ORIGINS=http://localhost:3000
PORT=8080
```

Server runs on `127.0.0.1:8080` by default.

## Architecture

Clean architecture with strict layer separation. Dependencies flow inward: Presentation → Application → Domain.

### Layer Responsibilities

**Domain** (`src/domain/`): Business entities, repository trait definitions, domain errors
- `Amount` is a newtype wrapper around `u64` for type safety
- Repository traits define the contract (`AccountRepository`, `UserRepository`)

**Application** (`src/application/`): Business logic
- `BankService<R>` is generic over repository type for testability
- `AuthService<R>` handles user registration, login, token generation

**Presentation** (`src/presentation/`): HTTP layer
- `AppState` contains both services and is injected via `web::Data<AppState>`
- Three middleware layers: `RequestIdMiddleware`, `TimingMiddleware`, `JwtAuthMiddleware`

**Data** (`src/data/`): Storage implementations
- Currently only in-memory with `Arc<RwLock<HashMap>>` for thread safety
- To add database: implement the repository traits in `domain/repository.rs`

**Infrastructure** (`src/infrastructure/`): Cross-cutting concerns
- `security.rs`: Argon2id password hashing (19MB, 2 iterations), HS256 JWT (1hr expiration)

### Critical Architectural Details

**Middleware Order** (in `main.rs`): CORS → Security Headers → JWT → Timing → RequestId

**JWT Authentication Flow**:
1. `JwtAuthMiddleware` checks if route is public (`/api/health`, `/api/auth/*`)
2. For protected routes, extracts Bearer token from Authorization header
3. Validates token using `validate_token()` from `infrastructure/security.rs`
4. On success, inserts `AuthenticatedUser { user_id }` into request extensions
5. Handlers can extract user via: `req.extensions().get::<AuthenticatedUser>()`

**Thread-Safe In-Memory Storage**:
- Uses `Arc<RwLock<HashMap<K, V>>>` pattern
- All async operations use `.read().await` for reads, `.write().await` for writes
- Data persists only during server lifetime

**Transfer Operation Limitation**: NOT atomic in current in-memory implementation. Performs sequential withdraw then deposit without transaction guarantees. Database implementation should wrap both in a transaction.

## Routes

Public (no auth):
- `POST /api/auth/register` - body: `{email, password}`
- `POST /api/auth/login` - returns `{access_token}` (JWT)
- `GET /api/health`

Protected (requires `Authorization: Bearer <token>`):
- `POST /api/accounts` - body: `{name}`
- `GET /api/accounts/{id}`
- `POST /api/accounts/{id}/deposit` - body: `{amount}`
- `POST /api/accounts/{id}/withdraw` - body: `{amount}`
- `POST /api/transfers` - body: `{from_account_id, to_account_id, amount}`

## Key Implementation Patterns

**Generic Services**: Services use `<R: Repository>` trait bounds. This allows different storage backends without changing business logic.

**Error Flow**: `DomainError` → `anyhow::Result` (service layer) → HTTP error responses (handlers)

**Logging**: All service methods use `#[instrument]` for automatic tracing spans. Operations log at `info!`, validations at `debug!`, internal steps at `trace!`.

**ID Generation**:
- Account IDs: `fastrand::u32(..)` (simple but collision-prone)
- User IDs: `uuid::v4()` (globally unique)

## Testing

- Unit tests embedded in modules with `#[cfg(test)]`
- Integration tests in `tests/` directory spawn real Actix-web servers
- Use `actix_web::test::TestRequest` for HTTP integration tests

## Security Notes

- Passwords hashed with Argon2id (memory-hard, ~50-150ms)
- JWT tokens expire after 1 hour, 60-second validation leeway
- All protected routes validated by `JwtAuthMiddleware` before reaching handlers
