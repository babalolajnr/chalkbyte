-- Create users table
create extension if not exists "uuid-ossp";

create table users (
    id uuid primary key default uuid_generate_v4(),
    first_name varchar not null,
    last_name varchar not null,
    email varchar unique not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

-- create index on email for faster lookups
create index idx_users_email on users(email);

-- create index on created_at for pagination
create index idx_users_created_at on users(created_at);
