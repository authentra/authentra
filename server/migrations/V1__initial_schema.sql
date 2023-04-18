create table users(
    uid uuid primary key,
    name varchar(32) unique check (name = lower(name)),
    password varchar(128),
    created_at timestamp,
    last_login timestamp
);