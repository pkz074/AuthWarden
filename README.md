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
- Docker Compose for the full local app stack

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

## Full Docker Stack

Build and run AuthWarden, PostgreSQL, and Redis:

```sh
docker compose up --build
```

The app is available at `http://127.0.0.1:8080`.

## Local Development

Start only PostgreSQL and Redis:

```sh
docker compose up -d postgres redis
```

Start the app locally:

```sh
DATABASE_URL=postgres://authwarden:authwarden@localhost:5432/authwarden \
REDIS_URL=redis://127.0.0.1:6379 \
JWT_SECRET=replace-this-with-a-long-secret \
cargo run
```

Migrations run automatically when the app starts.

The server listens on `http://127.0.0.1:8080` by default.

Manual migration command, if needed:

```sh
DATABASE_URL=postgres://authwarden:authwarden@localhost:5432/authwarden sqlx migrate run
```

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

Phase 1 and Phase 2 features are implemented and tested with Docker-backed integration coverage.
Phase 3 includes Docker Compose, CI, GHCR image publishing, and Kubernetes deployment manifests.

## Kubernetes

Kubernetes manifests live in `k8s/`. They deploy the AuthWarden app and assume PostgreSQL and Redis are available as external or separately managed services.
Ingress and TLS deployment notes are in `k8s/ingress-tls.md`.

```sh
cp k8s/secret.example.yaml k8s/secret.yaml
kubectl apply -k k8s
```

## Tests

CI runs formatting, Clippy, unit tests, the Docker-backed integration flow, and a Docker image build on pushes and pull requests.
Pushes to `main` publish the Docker image to GitHub Container Registry as `ghcr.io/pkz074/authwarden`.

Run the unit tests:

```sh
cargo test --offline
```

Run the Docker-backed integration flow:

```sh
docker compose up -d
cargo test --test auth_flow -- --ignored
```
