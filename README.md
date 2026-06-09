# Task Management API (Rust)

A REST API backend built with Axum implementing a task management system with JWT-based authentication, two-factor authentication (2FA), and Redis caching.

## API Documentation (Swagger / OpenAPI)

An OpenAPI specification is available in the root folder at [openapi.yaml](file:///home/vatsal/assesment-rust/openapi.yaml). You can import this file directly into Swagger Editor, Postman, or any other OpenAPI-compatible tool to inspect, visualize, and interact with the endpoints.

## Tech Stack

- **Rust** (2024 edition)
- **Axum** — HTTP web framework
- **SQLx** — Async PostgreSQL driver with migration support
- **Redis** — High-performance caching layer
- **Argon2** — Cryptographically secure password/code hashing
- **JWT** — Authentication tokens with `jsonwebtoken` (using `rust_crypto` backend)

---

## Getting Started

### Prerequisites

- Rust 1.80+ / 1.96.0 compatible
- PostgreSQL 16+
- Redis 7+

### 1. Database and Cache Setup

Ensure PostgreSQL and Redis are running. You can quickly launch Redis via Docker Compose:

```bash
docker compose up -d redis
```

Configure your local environment variables in a `.env` file (or keep the default fallback values if running on standard localhost ports). Example `.env` format:

```env
DATABASE_URL=postgres://postgres:password@localhost:5432/assessment_db
REDIS_URL=redis://127.0.0.1:6379
JWT_SECRET=SUPER_SECRET_SIGNING_KEY_12345_DONOTUSEINPRODUCTION
```

If you don't have the database created on your native PostgreSQL instance, create it:

```bash
psql -U postgres -c "CREATE DATABASE assessment_db;"
```

### 2. Run Database Migrations

Migrations will automatically execute on application startup. You can also run them using `sqlx-cli` if installed.

### 3. Run the Server

```bash
cargo run
```
The server will start at `http://127.0.0.1:8080`.

---

## Running the Integration Test Suite

A complete integration test has been added to cover the entire 11-step assessment workflow. To run the tests:

```bash
cargo test
```

The test suite automatically handles database truncation, seeds the admin and staff users, runs the full 2FA process, creates 5 tasks, assigns 3 tasks, performs role-based authorization check (getting a `403 Forbidden` for task creation as staff), and verifies the cache hit logic on Redis.

---

## Validation Flow & API Shape

### 1. Seed Users
```bash
curl -X POST http://localhost:8080/seed/users
```
Creates `admin@company.com` (Admin) and `jamesbond@example.com` (Staff).

### 2. Login as Admin
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"email":"admin@company.com","password":"AdminPassword123"}' \
  http://localhost:8080/auth/login
```
Returns a `login_challenge_id`.

### 3. Retrieve 2FA Code (Development Backdoor)
```bash
curl http://localhost:8080/dev/email-logs/latest
```
Returns the plain verification code.

### 4. Verify 2FA
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"login_challenge_id":"<CHALLENGE_ID>","code":"<CODE>"}' \
  http://localhost:8080/auth/verify-2fa
```
Returns the JWT access token.

### 5. Create 5 Tasks (Admin Only)
Call `POST /tasks` 5 times with header `Authorization: Bearer <ADMIN_TOKEN>`.

### 6. Assign 3 Tasks (Admin Only)
```bash
curl -X POST \
  -H "Authorization: Bearer <ADMIN_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{"user_id":"<JAMES_BOND_UUID>","task_ids":["<ID_1>","<ID_2>","<ID_3>"]}' \
  http://localhost:8080/tasks/assign
```

### 7. Login and Verify 2FA as James Bond
Follow steps 2-4 using `jamesbond@example.com` and password `ShakenNotStirred` to receive a `JAMES_BOND_TOKEN`.

### 8. Attempt Task Creation as James Bond (Fails)
```bash
curl -i -X POST \
  -H "Authorization: Bearer <JAMES_BOND_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{"title":"Illegal Task","priority":"medium"}' \
  http://localhost:8080/tasks
```
Returns `403 Forbidden`.

### 9. Retrieve Assigned Tasks as James Bond
```bash
curl -X GET -H "Authorization: Bearer <JAMES_BOND_TOKEN>" http://localhost:8080/tasks/view-my-tasks
```

#### Final Validation Response (First Request - Cache Miss)
```json
{
  "user": {
    "email": "jamesbond@example.com",
    "role": "staff"
  },
  "tasks": [
    {
      "id": "adf958dd-a3ff-4f62-afd8-be75429db9c0",
      "title": "Task 1",
      "status": "todo",
      "priority": "high",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "098e945c-15ba-4f24-bc67-ab17ef9bc289",
      "title": "Task 2",
      "status": "todo",
      "priority": "medium",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "e2cd7781-a67b-4d43-a611-9e23ba9c5cf1",
      "title": "Task 3",
      "status": "todo",
      "priority": "low",
      "assigned_to": "jamesbond@example.com"
    }
  ],
  "summary": {
    "total_assigned_tasks": 3
  },
  "cache": {
    "hit": false
  }
}
```

Calling the exact same endpoint again yields:

#### Second Request - Cache Hit
```json
{
  "user": {
    "email": "jamesbond@example.com",
    "role": "staff"
  },
  "tasks": [
    {
      "id": "adf958dd-a3ff-4f62-afd8-be75429db9c0",
      "title": "Task 1",
      "status": "todo",
      "priority": "high",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "098e945c-15ba-4f24-bc67-ab17ef9bc289",
      "title": "Task 2",
      "status": "todo",
      "priority": "medium",
      "assigned_to": "jamesbond@example.com"
    },
    {
      "id": "e2cd7781-a67b-4d43-a611-9e23ba9c5cf1",
      "title": "Task 3",
      "status": "todo",
      "priority": "low",
      "assigned_to": "jamesbond@example.com"
    }
  ],
  "summary": {
    "total_assigned_tasks": 3
  },
  "cache": {
    "hit": true
  }
}
```
