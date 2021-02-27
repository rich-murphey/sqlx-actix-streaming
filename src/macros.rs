// -*- compile-command: "cargo check --features runtime-tokio-rustls,postgres"; -*-

#[macro_export]
macro_rules! json_response [
    ( $pool:expr,
      $params:ident,
      $query:expr
    ) => ({
        HttpResponse::Ok()
            .content_type("application/json")
            .streaming(
                $crate::ByteStream::new(
                    $crate::RowStream::build(
                        ($pool, $params),
                        move |(pool, $params)| {
                            { $query }.fetch(pool)
                        }
                    ),
                    |buf: &mut BytesWriter, rec| {
                        serde_json::to_writer(buf, rec)
                            .map_err(actix_web::error::ErrorInternalServerError)
                    },
                )
            )
    });
];

#[macro_export]
macro_rules! byte_stream [
    ( $pool:expr,
      $params:ident,
      $query:expr
    ) => ({
        $crate::ByteStream::new(
            $crate::RowStream::build(
                ($pool, $params),
                move |(pool, $params)| {
                    { $query }.fetch(pool)
                }
            ),
            |buf: &mut BytesWriter, rec| {
                serde_json::to_writer(buf, rec)
                    .map_err(actix_web::error::ErrorInternalServerError)
            },
        )
    });
];

#[macro_export]
macro_rules! json_response_alt [
    // Note: sqlx::query_as!() must have literal parameters, otherwise
    // it causes error: cannot return value referencing local data
    // `param`.
    ( $item_struct:path,
      $pool:expr,
      $sql:literal,
      $( $arg:literal ),*
    ) => ({
        HttpResponse::Ok()
            .content_type("application/json")
            .streaming(
                $crate::ByteStream::new(
                    $crate::RowStream::build(
                        $pool,
                        move |pool| {
                            sqlx::query_as!(
                                $item_struct,
                                $sql,
                                $( $arg, )*
                            )
                                .fetch(pool)
                        }
                    ),
                    |buf: &mut BytesWriter, rec: & $item_struct| {
                        serde_json::to_writer(buf, rec)
                            .map_err(actix_web::error::ErrorInternalServerError)
                    },
                )
            )
    });
    ( $item_struct:path,
      $pool:expr,
      $params:ident,
      $sql:literal,
      $( $arg:expr ),*
    ) => ({
        HttpResponse::Ok()
            .content_type("application/json")
            .streaming(
                $crate::ByteStream::new(
                    $crate::RowStream::build(
                        ($pool, $params),
                        move |(pool, $params)| {
                            sqlx::query_as!(
                                $item_struct,
                                $sql,
                                $( $arg, )*
                            )
                                .fetch(pool)
                        }
                    ),
                    |buf: &mut BytesWriter, rec: & $item_struct| {
                        serde_json::to_writer(buf, rec)
                            .map_err(actix_web::error::ErrorInternalServerError)
                    },
                )
            )
    });
    ( $item_struct:path,
      $pool:expr,
      $sql:expr,
      $( $arg:expr ),*
    ) => ({
        HttpResponse::Ok()
            .content_type("application/json")
            .streaming(
                $crate::ByteStream::new(
                    $crate::RowStream::build(
                        ($pool, $sql),
                        move |(pool, sql)| {
                            sqlx::query_as::<_, $item_struct>(sql)
                                $( .bind($arg) )*
                                .fetch(pool)
                        }
                    ),
                    |buf: &mut BytesWriter, rec: & $item_struct| {
                        serde_json::to_writer(buf, rec)
                            .map_err(actix_web::error::ErrorInternalServerError)
                    },
                )
            )
    });
];

#[macro_export]
macro_rules! json_stream [
    ( $item_struct:path,
      $pool:expr,
      $sql:literal,
      $( $arg:literal ),*
    ) => ({
        $crate::ByteStream::new(
            $crate::RowStream::build(
                ($pool,),
                move |(pool,)| {
                    sqlx::query_as!(
                        $item_struct,
                        $sql,
                        $( $arg, )*
                    )
                        .fetch(pool)
                }
            ),
            |buf: &mut BytesWriter, row| {
                serde_json::to_writer(buf, row)
                    .map_err(actix_web::error::ErrorInternalServerError)
            },
        )
    });
    ( $item_struct:path,
      $pool:expr,
      $sql:expr,
      $( $arg:expr ),*
    ) => ({
        $crate::ByteStream::new(
            $crate::RowStream::build(
                ($pool, $sql),
                move |(pool, sql)| {
                    sqlx::query_as::<_, $item_struct>(sql)
                        $( .bind($arg) )*
                        .fetch(pool)
                }
            ),
            |buf: &mut BytesWriter, row| {
                serde_json::to_writer(buf, row)
                    .map_err(actix_web::error::ErrorInternalServerError)
            },
        )
    });
];

#[macro_export]
macro_rules! query_stream [
    ( $pool:expr,
      $sql:literal,
      $( $arg:expr ),*
    ) => ({
        $crate::RowStream::make(
            $pool,
            move |pool| {
                sqlx::query(sql)
                    $( .bind($arg) )*
                    .fetch(pool)
            }
        )
    });
    ( $item_struct:path,
      $pool:expr,
      $sql:expr,
      $( $arg:expr ),*
    ) => ({
        $crate::RowStream::make(
            ($pool, $sql),
            move |(pool, sql)| {
                sqlx::query(sql)
                    $( .bind($arg) )*
                    .fetch(pool)
            }
        )
    });
];

#[macro_export]
macro_rules! query_as_stream [
    ( $item_struct:path,
      $pool:expr,
      $sql:literal,
      $( $arg:literal ),*
    ) => ({
        $crate::RowStream::make(
            ($pool, $sql.to_string()),
            |(pool, _sql)| {
                sqlx::query_as!(
                    $item_struct,
                    $sql,
                    $( $arg ),*
                )
                    .fetch(pool)
            }
        )
    });
    ( $item_struct:path,
      $pool:expr,
      $sql:expr,
      $( $arg:expr ),*
    ) => ({
        $crate::RowStream::make(
            ($pool, $sql),
            move |(pool, sql)| {
                sqlx::query_as::<_, $item_struct>(sql)
                    $( .bind($arg) )*
                    .fetch(pool)
            }
        )
    });
];

#[macro_export]
macro_rules! query_as_byte_stream [
    ( $item_struct:path,
      $pool:expr,
      $sql:literal,
      $fn:expr,
      $( $arg:literal ),*
    ) => ({
        $crate::byte_stream(
            $crate::RowStream::make(
                $pool,
                move |pool| {
                    sqlx::query_as!(
                        $item_struct,
                        $sql,
                        $( $arg ),*
                    )
                        .fetch(pool)
                }
            ),
            $fn,
        )
    });
    ( $item_struct:path,
      $pool:expr,
      $sql:expr,
      $fn:expr,
      $( $arg:expr ),*
    ) => ({
        $crate::sql_byte_stream(
            $crate::RowStream::make(
                ($pool, $sql),
                move |(pool, sql)| {
                    sqlx::query_as::<_, $item_struct>(sql)
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
    ( $pool:expr,
      $sql:literal,
      $fn:expr,
      $( $arg:literal ),*
    ) => ({
        $crate::byte_stream(
            $crate::RowStream::make(
                $pool,
                move |pool| {
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
    ( $pool:expr,
      $sql:expr,
      $fn:expr,
      $( $arg:expr ),*
    ) => ({
        $crate::sql_byte_stream(
            $crate::RowStream::make(
                ($pool, $sql),
                move |(pool, sql)| {
                    sqlx::query::<_>(sql)
                        $( .bind($arg) )*
                        .fetch(pool)
                },
            ),
            $fn,
        )
    });
];
