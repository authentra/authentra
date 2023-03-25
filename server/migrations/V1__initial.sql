create table users
(
    uid                  uuid                     default gen_random_uuid() primary key not null,
    name                 varchar(32) unique                                             not null check ( name = lower(name) ),
    email                varchar(64) unique,
    display_name         varchar(32),
    password             varchar(255)                                                   not null,
    password_change_date timestamp with time zone default now()                         not null,
    administrator        boolean                  default false                         not null
);

create unique index users_name_lower on users ((lower(name)));

create table sessions
(
    uid     char(96) primary key,
    user_id uuid references users
);

create type policy_kind as enum ('password_expiry', 'password_strength', 'expression');

create table policies
(
    uid              serial primary key,
    slug             varchar(128) not null unique check ( slug = lower(slug) ),
    kind             policy_kind  not null,
    password_max_age int4,
    expression       text
);

create unique index policy_slug ON policies ((lower(slug)));

create table policy_bindings
(
    uid           serial primary key,
    enabled       bool not null,
    negate_result bool not null,
    policy        int4 not null references policies
);

create type prompt_kind as enum ('username', 'email', 'password', 'text', 'text_read_only', 'signed_number', 'unsigned_number', 'checkbox', 'switch', 'date', 'date_time', 'seperator', 'static', 'locale');

create table prompts
(
    uid         serial primary key,
    field_key   varchar(32) not null,
    label       varchar(32) not null,
    kind        prompt_kind not null,
    placeholder varchar(128),
    required    bool        not null,
    help_text   varchar(128)
);

create type stage_kind as enum ('deny', 'prompt', 'identification', 'user_login', 'user_logout', 'user_write', 'password', 'consent');

create type consent_mode as enum ('always', 'once', 'until');

create table consent_stages
(
    uid   serial primary key,
    mode  consent_mode not null,
    until int4
);

create type userid_field as enum ('email', 'name', 'uuid');

create table identification_stages
(
    uid    serial primary key,
    fields userid_field[] not null
);

create table stages
(
    uid                           serial primary key,
    slug                          varchar(128) not null check ( slug = lower(slug) ),
    kind                          stage_kind   not null,
    timeout                       int4         not null,
    identification_password_stage int4 references stages,
    identification_stage          int4 references stages,
    consent_stage                 int4 references consent_stages
);

create unique index stage_slug on stages ((lower(slug)));

create table stage_prompt_bindings
(
    prompt   int4 not null references prompts,
    stage    int4 not null references stages,
    ordering int2 not null,
    primary key (prompt, stage)
);

create index stage_prompt_bindings_stage on stage_prompt_bindings (stage);

create type authentication_requirement as enum ('required', 'none', 'superuser', 'ignored');
create type flow_designation as enum ('invalidation', 'authentication', 'authorization', 'enrollment', 'recovery', 'unenrollment', 'configuration');

create table flows
(
    uid            serial primary key,
    slug           varchar(128)               not null check ( slug = lower(slug) ),
    title          varchar(128)               not null unique,
    designation    flow_designation           not null,
    authentication authentication_requirement not null

);

create unique index flow_slug on flows ((lower(slug)));

create table flow_entries
(
    uid      serial primary key,
    flow     int4 not null references flows on delete cascade,
    stage    int4 not null references stages on delete cascade,
    ordering int2 not null
);

create unique index on flow_entries (flow, stage, ordering);

create table flow_bindings
(
    policy        int4 not null references policies on delete cascade,
    flow          int4 references flows on delete cascade,
    entry         int4 references flow_entries on delete cascade,
    group_binding uuid,
    user_binding  uuid references users on delete cascade,

    ordering      int2 not null,
    enabled       bool not null,
    negate_result bool not null,
    primary key (policy, flow, entry)
);


create table providers
(
    uid          serial primary key,
    slug         varchar(64) not null check ( slug = lower(slug) ),
    display_name varchar(64) not null
);

create unique index provider_slug on providers ((lower(slug)));

create table applications
(
    uid          serial primary key,
    slug         varchar(64) not null check ( slug = lower(slug) ),
    display_name varchar(64) not null,
    provider     int4        not null references providers
);

create unique index application_slug on applications ((lower(slug)));

create table tenants
(
    uid                 serial primary key,
    host                varchar(255) not null unique,
    is_default          bool         not null,
    title               varchar(64)  not null,
    logo                varchar(255) not null,
    favicon             varchar(255) not null,

    invalidation_flow   int4         references flows on delete set null,
    authentication_flow int4         references flows on delete set null,
    authorization_flow  int4         references flows on delete set null,
    enrollment_flow     int4         references flows on delete set null,
    recovery_flow       int4         references flows on delete set null,
    unenrollment_flow   int4         references flows on delete set null,
    configuration_flow  int4         references flows on delete set null
);