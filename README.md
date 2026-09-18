# AuthWarden

AuthWarden is a Rust authentication backend built with Axum, PostgreSQL, Redis, JWT access tokens, refresh-token rotation, password login, GitHub/Google OAuth, audit logs, metrics, Docker, CI, and Kubernetes manifests.

It is intentionally framework-light so the auth flow is easy to read and reason about.

## Architecture

```text
client
  -> Axum router
     -> handlers      HTTP parsing and responses
     -> extractors    bearer-token authentication
     -> services      passwords, JWTs, refresh tokens, OAuth, client identity
     -> db            SQLx queries
     -> middleware    request IDs, CORS, rate limits, metrics, security headers

PostgreSQL: users, refresh sessions, OAuth accounts, audit logs
Redis: OAuth state, revoked refresh-token hashes, rate limits, login lockout
```

## Auth Model

- Passwords are hashed with Argon2id.
- Access tokens are short-lived JWTs signed with `JWT_SECRET`.
- Refresh tokens are returned once; only SHA-256 hashes are stored.
- Refresh rotation is transactional.
- OAuth state is stored in Redis and consumed on callback.
- OAuth user/account linking is transactional.
- Sensitive routes are rate-limited through Redis.
- Password lockout is scoped by email and client identity.
- `/metrics` can require a bearer token with `METRICS_TOKEN`.

## Endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/register` | Create password user |
| `POST` | `/login` | Issue access and refresh tokens |
| `POST` | `/refresh` | Rotate refresh token |
| `POST` | `/logout` | Revoke refresh session |
| `GET` | `/me` | Authenticated user profile |
| `GET` | `/auth/github` | Start GitHub OAuth |
| `GET` | `/auth/google` | Start Google OAuth |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/health`, `/health/db` | Health checks |

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `APP_HOST` | `127.0.0.1` | HTTP bind host |
| `APP_PORT` | `8080` | HTTP bind port |
| `DATABASE_URL` | local Docker URL | PostgreSQL connection |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection |
| `JWT_SECRET` | required | JWT signing secret |
| `TRUST_PROXY_HEADERS` | `false` | Trust proxy IP headers only behind known ingress |
| `METRICS_TOKEN` | unset | Optional bearer token for `/metrics` |
| `OAUTH_HTTP_TIMEOUT_SECONDS` | `5` | OAuth provider timeout |
| `CORS_ALLOWED_ORIGINS` | empty | Comma-separated allowed origins |
| `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_REDIRECT_URI` | unset | GitHub OAuth |
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_REDIRECT_URI` | unset | Google OAuth |

OAuth providers are disabled when any required provider variable is missing.

## Run Locally

```sh
docker compose up -d postgres redis

DATABASE_URL=postgres://authwarden:authwarden@localhost:5432/authwarden \
REDIS_URL=redis://127.0.0.1:6379 \
JWT_SECRET=replace-this-with-a-long-secret \
cargo run
```

The app listens on `http://127.0.0.1:8080`. Migrations run on startup.

Run the full local stack:

```sh
docker compose up --build
```

## Example

```sh
curl -i -X POST http://127.0.0.1:8080/register \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  --data-urlencode 'email=user@example.com' \
  --data-urlencode 'password=Password123'

curl -s -X POST http://127.0.0.1:8080/login \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  --data-urlencode 'email=user@example.com' \
  --data-urlencode 'password=Password123'

curl -s http://127.0.0.1:8080/me \
  -H "Authorization: Bearer ACCESS_TOKEN"
```

## Tests

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
docker compose up -d postgres redis
cargo test --tests -- --ignored
make audit
make coverage
make coverage-all
```

Current line coverage:

- Fast tests: `53.43%`
- Full Docker-backed tests: `83.07%`

## Deployment

Kubernetes manifests live in `k8s/` and assume PostgreSQL and Redis are provided separately.

```sh
cp k8s/secret.example.yaml k8s/secret.yaml
kubectl apply -k k8s
```

CI runs formatting, Clippy, dependency audit, unit tests, Docker-backed integration tests, fast coverage, and Docker image build. Pushes to `main` publish `ghcr.io/pkz074/authwarden`.

## Notes

- API responses use JSON errors: `{"error":"..."}`.
- Browser auth currently returns bearer tokens as JSON; secure cookies are a future web-app direction.
- `cargo audit` ignores `RUSTSEC-2023-0071` because `rsa` is not in the active dependency tree; it appears through SQLx optional lockfile metadata.
