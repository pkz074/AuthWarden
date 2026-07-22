# AuthWarden

AuthWarden is a Rust authentication service built as a learning project and portfolio backend. It implements email/password auth with PostgreSQL-backed users, Argon2 password hashing, JWT access tokens, refresh-token rotation, Redis-backed revocation caching, and audit logging.

## Features

- Email/password registration and login
- Argon2id password hashing
- PostgreSQL user and refresh-session storage
- JWT access tokens
- Refresh tokens stored only as SHA-256 hashes
- Transactional refresh-token rotation
- Redis cache for revoked refresh-token hashes
- Logout with refresh-session revocation
- Audit logs for register, login, refresh, and logout
- Basic HTML login page scaffold
- Docker Compose for local PostgreSQL and Redis

## Stack

- Rust 2024
- Axum
- Tokio
- SQLx
- PostgreSQL
- Redis
- Argon2
- JSON Web Tokens
- Docker

## Local Setup

Start PostgreSQL and Redis:

```sh
docker compose up -d
```

Run database migrations:

```sh
DATABASE_URL=postgres://authwarden:authwarden@localhost:5432/authwarden sqlx migrate run
```

Start the app:

```sh
DATABASE_URL=postgres://authwarden:authwarden@localhost:5432/authwarden \
REDIS_URL=redis://127.0.0.1:6379 \
JWT_SECRET=replace-this-with-a-long-secret \
cargo run
```

The server listens on `http://127.0.0.1:8080` by default.

## Environment

| Variable | Default | Description |
| --- | --- | --- |
| `APP_HOST` | `127.0.0.1` | Host address for the Axum server |
| `APP_PORT` | `8080` | Port for the Axum server |
| `DATABASE_URL` | local Docker Postgres URL | PostgreSQL connection string |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection string |
| `JWT_SECRET` | required | HMAC secret used to sign JWT access tokens |

## Endpoints

| Method | Path | Description |
| --- | --- | --- |
| `GET` | `/` | Login page scaffold |
| `GET` | `/health` | Basic health check |
| `GET` | `/health/db` | PostgreSQL health check |
| `POST` | `/register` | Create a user |
| `POST` | `/login` | Issue access and refresh tokens |
| `POST` | `/refresh` | Rotate a refresh token and issue a new token pair |
| `POST` | `/logout` | Revoke a refresh session |
| `GET` | `/me` | Return the authenticated user |

## Example Requests

Register:

```sh
curl -i -X POST http://127.0.0.1:8080/register \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  --data-urlencode 'email=user@example.com' \
  --data-urlencode 'password=Password123'
```

Login:

```sh
curl -s -X POST http://127.0.0.1:8080/login \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  --data-urlencode 'email=user@example.com' \
  --data-urlencode 'password=Password123'
```

## Current Status

Phase 1 and Phase 2 features are implemented and manually tested with Docker. Automated integration tests are planned next.
