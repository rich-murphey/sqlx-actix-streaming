// Copyright (c) 2020 Carey Richard Murphey.
pub use std::io::Write;

#[macro_export]
macro_rules! query_stream [
    (
        $pool:expr,
        $sql:literal,
        $( $arg:expr ),*
    ) => ({
        RowStream::pin(
            $pool,
            |pool| {
                sqlx::query(sql)
                    $( .bind($arg) )*
                    .fetch(pool)
            }
        )
    });
    (
        $struct_name:ident,
        $pool:expr,
        $sql:expr,
        $( $arg:expr ),*
    ) => ({
        RowWStmtStream::pin(
            $pool,
            $sql,
            |pool,sql| {
                sqlx::query(sql)
                    $( .bind($arg) )*
                    .fetch(pool)
            }
        )
    });
];

#[macro_export]
macro_rules! query_as_stream [
    (
        $struct_name:ident,
        $pool:expr,
        $sql:literal,
        $( $arg:expr ),*
    ) => ({
        RowWStmtStream::pin(
            $pool,
            $sql.to_string(),
            |pool,_sql| {
                sqlx::query_as!(
                    $struct_name,
                    $sql,
                    $( $arg ),*
                )
                    .fetch(pool)
            }
        )
    });
    (
        $struct_name:ident,
        $pool:expr,
        $sql:expr,
        $( $arg:expr ),*
    ) => ({
        RowWStmtStream::pin(
            $pool,
            $sql,
            |pool,sql| {
                sqlx::query_as::<sqlx::postgres::Postgres, $struct_name>(sql)
                    $( .bind($arg) )*
                    .fetch(pool)
            }
        )
    });
];

#[macro_export]
macro_rules! query_as_byte_stream [
    (
        $struct_name:ident,
        $pool:expr,
        $sql:literal,
        $fn:expr,
        $( $arg:expr ),*
    ) => ({
        ByteStream::pin(
            RowStream::pin(
                $pool,
                |pool| {
                    sqlx::query_as!(
                        $struct_name,
                        $sql,
                        $( $arg ),*
                    )
                        .fetch(pool)
                }
            ),
            $fn,
        )
    });
    (
        $struct_name:ident,
        $pool:expr,
        $sql:expr,
        $fn:expr,
        $( $arg:expr ),*
    ) => ({
        ByteStream::pin(
            RowWStmtStream::pin(
                $pool,
                $sql,
                |pool,sql| {
                    sqlx::query_as::<sqlx::postgres::Postgres, $struct_name>(sql)
                        $( .bind($arg) )*
                        .fetch(pool)
                },
            ),
            $fn,
        )
    });
];

#[macro_export]
macro_rules! query_byte_stream [
    (
        $pool:expr,
        $sql:literal,
        $fn:expr,
        $( $arg:expr ),*
    ) => ({
        ByteStream::pin(
            RowStream::pin(
                $pool,
                |pool| {
                    sqlx::query!(
                        $sql,
                        $( $arg ),*
                    )
                        .fetch(pool)
                }
            ),
            $fn,
        )
    });
    (
        $pool:expr,
        $sql:expr,
        $fn:expr,
        $( $arg:expr ),*
    ) => ({
        ByteStream::pin(
            RowWStmtStream::pin(
                $pool,
                $sql,
                |pool,sql| {
                    sqlx::query::<sqlx::postgres::Postgres>(sql)
                        $( .bind($arg) )*
                        .fetch(pool)
                },
            ),
            $fn,
        )
    });
];
