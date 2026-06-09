# Task Management API (Rust)

A REST API backend built with Axum implementing a task management system with JWT-based authentication, two-factor authentication (2FA), and Redis caching.

> **Note:** This project is a work-in-progress assessment. It has not been fully tested.

## Tech Stack

- **Rust** (2024 edition)
- **Axum** — HTTP framework
- **SQLx** — Async PostgreSQL driver
- **Redis** — Caching layer
- **Argon2** — Password/code hashing
- **JWT** — Authentication tokens

## API Routes

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/seed/users` | None | Seed admin + staff users |
| POST | `/auth/login` | None | Login, get 2FA challenge |
| GET | `/dev/email-logs/latest` | None | Read latest 2FA code (dev) |
| POST | `/auth/verify-2fa` | None | Verify 2FA, get JWT |
| POST | `/tasks` | JWT (Admin) | Create a task |
| POST | `/tasks/assign` | JWT (Admin) | Assign tasks to user |
| GET | `/tasks/view-my-tasks` | JWT | View assigned tasks (cached) |

## Getting Started

### Prerequisites

- Rust 1.78+
- PostgreSQL 16
- Redis 7

### With Docker Compose

```bash
docker compose up --build
```

Server runs at `http://localhost:8080`.

### Without Docker

1. Copy `.env.example` to `.env` and configure your database/Redis URLs.
2. Run migrations against your PostgreSQL instance.
3. Start the server:

```bash
cargo run
```

### Seed Users

```bash
curl -X POST http://localhost:8080/seed/users
```

| Email | Password | Role |
|-------|----------|------|
| admin@company.com | AdminPassword123 | Admin |
| jamesbond@example.com | ShakenNotStirred | Staff |

## Acknowledgments

Gemini helped with this project: https://gemini.google.com/share/57c3028e6398
