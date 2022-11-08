CREATE TABLE IF NOT EXISTS heroes (
    id bigserial PRIMARY KEY,
    first_seen timestamp(0) NOT NULL DEFAULT NOW(),
    name text NOT NULL,
    can_fly boolean NOT NULL DEFAULT false,
    realName text NULL,
    abilities text[] NOT NULL,
    version integer NOT NULL DEFAULT 1
);
