-- `gen_random_uuid()` wasn't added until Postgres 13
create extension if not exists "uuid-ossp";

create table "user"
(
    user_id  uuid primary key default uuid_generate_v1mc(),
    username text unique not null
);

create table post (
    post_id uuid primary key default uuid_generate_v1mc(),
    user_id uuid not null references "user"(user_id),
    content text not null,
    created_at timestamptz default now()
);

create index on post(created_at desc);

create table comment (
    comment_id uuid primary key default uuid_generate_v1mc(),
    post_id uuid not null references post(post_id),
    user_id uuid not null references "user"(user_id),
    content text not null,
    created_at timestamptz not null default now()
);

create index on comment(created_at desc);
