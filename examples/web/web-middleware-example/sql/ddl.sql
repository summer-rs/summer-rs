create sequence if not exists seq_error_log;

create table if not exists error_log (
    id bigint primary key default nextval('seq_error_log'),
    msg varchar(255) not null,
    created_at timestamp not null
);