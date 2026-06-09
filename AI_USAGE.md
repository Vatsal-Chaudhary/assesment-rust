# AI Usage Disclosure

## AI Tools Used
- **Google Gemini 3.5 Flash** (via Antigravity pair programming assistant)

## Automated Contributions
- Identified the deprecated `async_trait` macro usage in Axum v0.8.x and refactored the `FromRequestParts` implementation in [src/handlers/mod.rs](file:///home/vatsal/assesment-rust/src/handlers/mod.rs) to use native Rust async traits.
- Added the `rust_crypto` feature to `jsonwebtoken` in [Cargo.toml](file:///home/vatsal/assesment-rust/Cargo.toml) to prevent runtime panics regarding missing cryptography providers.
- Cleaned up compiler warnings for unused imports in [src/handlers/tasks.rs](file:///home/vatsal/assesment-rust/src/handlers/tasks.rs), [src/repository/task_repo.rs](file:///home/vatsal/assesment-rust/src/repository/task_repo.rs), and [src/handlers/mod.rs](file:///home/vatsal/assesment-rust/src/handlers/mod.rs).
- Extracted routing and shared state definitions into [src/lib.rs](file:///home/vatsal/assesment-rust/src/lib.rs) (converting the project into a library + binary crate) to make components testable.
- Generated a complete, end-to-end integration test suite in [tests/integration_test.rs](file:///home/vatsal/assesment-rust/tests/integration_test.rs) mapping out the 11-step validation workflow.

## Manual Modifications & Verification
- Audited the PostgreSQL migrations in [migrations/20260609031240_create_user.up.sql](file:///home/vatsal/assesment-rust/migrations/20260609031240_create_user.up.sql) to add `full_name`, `hashed_password`, `created_by_id`, and `updated_at` matching the requested minimum data model.
- Restructured SQL queries in user and task repository files to support the database schema changes.
- Tested and ran the test suite (`cargo test`) to verify correctness of the 2FA authentication, cache hits/misses, and role-based access control flows.
