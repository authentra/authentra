create extension if not exists pgcrypto;

create type user_roles as enum ('logs', 'developer', 'admin');
create type internal_scopes as enum ('profile:read', 'profile:write');
create type application_kind as enum ('web-server', 'spa');
create type consent_mode as enum ('explicit', 'implicit');

create table users(
    id uuid not null primary key default gen_random_uuid(),
    name varchar(32) unique check (name = lower(name)),
    email varchar(255) unique,
    password varchar(128),
    active boolean not null default true,
    created_at timestamp default now(),
    roles user_roles[] default array[]::user_roles[],
    customer boolean not null,
    require_password_reset boolean default false not null
);

create table sessions(
    id uuid not null primary key default gen_random_uuid(),
    user_id uuid not null references users on delete cascade,
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
    scopes internal_scopes[] not null default array[]::internal_scopes[],
    allow_implicit_consent boolean not null default false
);

create table developer_allowed_groups(
    id varchar(32) primary key references application_groups
);

create table applications(
    id uuid not null primary key default gen_random_uuid(),
    name varchar(32) not null,
    owner uuid not null references users on delete cascade,
    system_application boolean not null default false,
    application_group varchar(32) not null references application_groups,
    kind application_kind not null,
    client_id varchar(64) default encode(gen_random_bytes(32), 'hex') not null,
    redirect_uri varchar(256)[] default array[]::varchar(256)[],
    consent_mode consent_mode not null,
    client_secret varchar(48)
);

create table oauth_sessions(
    id uuid not null primary key default gen_random_uuid(),
    user_id uuid not null references users on delete cascade,
    application uuid not null references applications on delete cascade
);

create table refresh_tokens(
    id varchar(96) default encode(gen_random_bytes(48), 'hex') primary key not null,
    session uuid not null references oauth_sessions on delete cascade,
    is_used boolean not null default false
);

create table access_token(
    id varchar(96) default encode(gen_random_bytes(48), 'hex') primary key not null,
    session uuid not null references oauth_sessions on delete cascade,
    refresh_token varchar(96) references refresh_tokens on delete cascade
);

create table consents(
    user_id uuid not null references users on delete cascade,
    application uuid not null references applications on delete cascade,
    given boolean not null default false,
    implicit boolean not null default false,
    primary key(user_id, application)
);