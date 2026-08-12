# AuthWarden

AuthWarden is a Rust authentication service built to demonstrate how a production-style auth backend is assembled from small, explicit pieces: password auth, OAuth provider login, JWT access tokens, refresh-token rotation, Redis-backed revocation checks, audit logging, Docker, CI, and Kubernetes manifests.

The project favors readable, framework-light Rust over hiding the architecture behind a large auth library.

## Architecture

AuthWarden is an Axum HTTP service with PostgreSQL as the source of truth and Redis as short-lived security storage.

```text
browser / API client
        |
        v
Axum router
        |
        +--> handlers      HTTP request parsing and responses
        +--> extractors    authenticated-user extraction from bearer JWTs
        +--> services      password hashing, JWTs, refresh tokens, OAuth provider calls
        +--> db            SQLx queries for users, sessions, OAuth accounts, audit logs
        +--> models        request, response, and database row types
        |
        +--> PostgreSQL    users, refresh sessions, OAuth accounts, audit logs
        +--> Redis         revoked refresh-token hashes and OAuth state values
```

### Auth Model

- Password users store Argon2id password hashes.
- OAuth-only users are allowed by storing `NULL` in `users.password_hash`.
- JWT access tokens are short-lived and signed with `JWT_SECRET`.
- Refresh tokens are returned once to the client, but only their SHA-256 hash is stored.
- Refresh rotation revokes the old session and creates the replacement session transactionally.
- Logout revokes the refresh session and caches the revoked token hash in Redis.
- OAuth state values are stored in Redis with a short TTL and consumed on callback to prevent replay.

## Features

- Email/password registration and login
- GitHub OAuth login
- Google OAuth login
- JWT-protected `/me` endpoint
- Refresh-token rotation
- Logout and refresh-session revocation
- Redis revocation cache
- Audit logs for auth/session events
- HTML login and register pages
- Docker Compose local stack
- GitHub Actions CI
- GHCR image publishing
- Kubernetes deployment manifests

## Stack

- Rust 2024
- Axum
- Tokio
- SQLx
- PostgreSQL
- Redis
- Argon2
- JSON Web Tokens
- Reqwest
- Docker
- Kubernetes

## Endpoints

| Method | Path | Description |
| --- | --- | --- |
| `GET` | `/` | Login page |
| `GET` | `/login` | Login page |
| `GET` | `/register` | Register page |
| `POST` | `/register` | Create a password user |
| `POST` | `/login` | Issue access and refresh tokens |
| `GET` | `/auth/github` | Start GitHub OAuth login |
| `GET` | `/auth/github/callback` | Complete GitHub OAuth login |
| `GET` | `/auth/google` | Start Google OAuth login |
| `GET` | `/auth/google/callback` | Complete Google OAuth login |
| `POST` | `/refresh` | Rotate a refresh token |
| `POST` | `/logout` | Revoke a refresh session |
| `GET` | `/me` | Return the authenticated user |
| `GET` | `/health` | Basic health check |
| `GET` | `/health/db` | PostgreSQL health check |

## Configuration

| Variable | Default | Description |
| --- | --- | --- |
| `APP_HOST` | `127.0.0.1` | HTTP bind host |
| `APP_PORT` | `8080` | HTTP bind port |
| `DATABASE_URL` | local Docker Postgres URL | PostgreSQL connection string |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection string |
| `JWT_SECRET` | required | HMAC secret for JWT access tokens |
| `GITHUB_CLIENT_ID` | unset | GitHub OAuth app client ID |
| `GITHUB_CLIENT_SECRET` | unset | GitHub OAuth app client secret |
| `GITHUB_REDIRECT_URI` | unset | GitHub OAuth callback URL |
| `GOOGLE_CLIENT_ID` | unset | Google OAuth app client ID |
| `GOOGLE_CLIENT_SECRET` | unset | Google OAuth app client secret |
| `GOOGLE_REDIRECT_URI` | unset | Google OAuth callback URL |

OAuth providers are disabled when their client ID, client secret, or redirect URI is missing.

## Local Development

Start PostgreSQL and Redis:

```sh
docker compose up -d postgres redis
```

Run the app locally:

```sh
DATABASE_URL=postgres://authwarden:authwarden@localhost:5432/authwarden \
REDIS_URL=redis://127.0.0.1:6379 \
JWT_SECRET=replace-this-with-a-long-secret \
cargo run
```

The server listens on `http://127.0.0.1:8080`.

Migrations run automatically on startup. To run them manually:

```sh
DATABASE_URL=postgres://authwarden:authwarden@localhost:5432/authwarden sqlx migrate run
```

## Docker

Run the full local stack:

```sh
docker compose up --build
```

Build only the app image:

```sh
docker build -t authwarden .
```

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

Use the returned access token:

```sh
curl -s http://127.0.0.1:8080/me \
  -H "Authorization: Bearer ACCESS_TOKEN"
```

## Tests

Run unit tests:

```sh
cargo test
```

Run the Docker-backed integration flow:

```sh
docker compose up -d postgres redis
cargo test --tests -- --ignored
```

Run the main local quality checks:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Generate line coverage:

```sh
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
make coverage
```

Run coverage including the Docker-backed ignored integration test:

```sh
docker compose up -d postgres redis
make coverage-all
```

## Deployment

Kubernetes manifests live in `k8s/`. They deploy the AuthWarden app and assume PostgreSQL and Redis are available as external or separately managed services.

```sh
cp k8s/secret.example.yaml k8s/secret.yaml
kubectl apply -k k8s
```

Ingress and TLS deployment notes are in `k8s/ingress-tls.md`.

CI runs formatting, Clippy, unit tests, the Docker-backed integration flow, and a Docker image build. Pushes to `main` publish the Docker image to GitHub Container Registry as `ghcr.io/pkz074/authwarden`.

## Next Hardening Work

- Security headers
- Request IDs and structured request logs
- Redis-backed rate limiting
- Password login lockout
- Explicit CORS policy
- Prometheus metrics
- Dependency security audit
