create type user_roles as enum ('logs','admin');

create table users(
    uid uuid not null primary key default gen_random_uuid(),
    email varchar(255) unique,
    name varchar(32) unique check (name = lower(name)),
    password varchar(128),
    disabled boolean not null default false,
    created_at timestamp default now(),
    last_login timestamp default null,
    roles user_roles[] default array[]::user_roles[]
);

create table sessions(
    id uuid not null primary key default gen_random_uuid(),
    user_id uuid not null references users,
    token varchar(255) not null unique,
    address inet,
    creation_time timestamp default now()
);