-- Up Migration

-- 1. Create Roles Enum
CREATE TYPE user_role AS ENUM ('admin', 'staff');

-- 2. Create Task Priority and Status Enums (To support your final API shape)
CREATE TYPE task_priority AS ENUM ('low', 'medium', 'high');
CREATE TYPE task_status AS ENUM ('todo', 'in_progress', 'done');

-- 3. Users Table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    full_name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    hashed_password VARCHAR(255) NOT NULL, -- Argon2 hash
    role user_role NOT NULL DEFAULT 'staff',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. Login Challenges Table (For 2FA isolation step)
CREATE TABLE login_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hashed_code VARCHAR(255) NOT NULL, -- Stored securely (not plain text)
    expires_at TIMESTAMPTZ NOT NULL,   -- Set to NOW() + INTERVAL '5 minutes'
    is_used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. Tasks Table
CREATE TABLE tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status task_status NOT NULL DEFAULT 'todo',
    priority task_priority NOT NULL DEFAULT 'medium',
    created_by_id UUID NOT NULL REFERENCES users(id),
    assigned_to_id UUID REFERENCES users(id) ON DELETE SET NULL, -- Maps James Bond
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 6. Simulated Development Email Logs Table 
-- Allows GET /dev/email-logs/latest to securely query plain text for testing, 
-- while production handles it strictly in memory/STDOUT.
CREATE TABLE dev_email_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    plain_code VARCHAR(6) NOT NULL,
    login_challenge_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance and cache-invalidation searches
CREATE INDEX idx_tasks_assigned_to ON tasks(assigned_to_id);
CREATE INDEX idx_challenges_lookup ON login_challenges(id, is_used);

-- Down Migration
-- DROP TABLE IF EXISTS dev_email_logs;
-- DROP TABLE IF EXISTS tasks;
-- DROP TABLE IF EXISTS login_challenges;
-- DROP TABLE IF EXISTS users;
-- DROP TYPE IF EXISTS task_status;
-- DROP TYPE IF EXISTS task_priority;
-- DROP TYPE IF EXISTS user_role;
