create extension if not exists pgcrypto;

create type user_roles as enum ('logs','admin');
create type internal_scopes as enum ('profile:read', 'profile:write');
create type application_kind as enum ('web-server', 'spa');

create table users(
    uid uuid not null primary key default gen_random_uuid(),
    email varchar(255) unique,
    name varchar(32) unique check (name = lower(name)),
    password varchar(128),
    disabled boolean not null default false,
    created_at timestamp default now(),
    last_login timestamp default null,
    roles user_roles[] default array[]::user_roles[],
    require_password_reset boolean default false not null
);

create table sessions(
    id uuid not null primary key default gen_random_uuid(),
    user_id uuid not null references users,
    token varchar(255) not null unique,
    address inet,
    creation_time timestamp default now()
);

create table settings(
    id boolean default true check (id),
    registration_enabled boolean default true
);

create table application_groups(
    id varchar(32) primary key check (id = lower(id)),
    scopes internal_scopes[] not null default array[]::internal_scopes[]
);

create table applications(
    id uuid not null primary key default gen_random_uuid(),
    name varchar(32) not null,
    application_group varchar(32) not null references application_groups(id),
    kind application_kind not null,
    client_id varchar(64) default encode(gen_random_bytes(32), 'hex') not null,
    redirect_uri varchar(256)[] default array[]::varchar(256)[],
    client_secret varchar(48)
);