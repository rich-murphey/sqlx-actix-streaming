use actix_web::{
    error::ErrorInternalServerError,
    web::{BufMut, BytesMut},
    *,
};
use futures::{future, stream, StreamExt};
use serde::*;
use sqlx::{postgres::*, prelude::*};
use sqlx_actix_streaming::*;

#[derive(Serialize, FromRow)]
pub struct WidgetRecord {
    pub id: i64,
    pub serial: i64,
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Serialize)]
pub struct WidgetParams {
    pub offset: i64,
    pub limit: i64,
}

#[post("/widgets")]
pub async fn widgets(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    json_response!(
        pool.as_ref().clone(),
        params,
        sqlx::query_as!(
            WidgetRecord,
            "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
            params.limit,
            params.offset
        )
    )
}

#[post("/widgets2")]
pub async fn widgets2(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            // this is a stream of text (Bytes) containing a JSON array of sqlx records
            ByteStream::new(
                RowStream::build(
                (pool.as_ref().clone(), params),
                move |(pool, params)| {
                    // this is a a stream of WidgetRecords that borrows pool and params
                    sqlx::query_as!(
                        WidgetRecord,
                        "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                        params.limit,
                        params.offset
                    )
                    .fetch(pool)
                }),
                |buf: &mut BytesWriter, record: &WidgetRecord| {
                    // this writes a WidgetRecords as JSON text to the output buffer
                    serde_json::to_writer(buf, record).map_err(ErrorInternalServerError)
                },
            ),
        )
}

// NOTE: this is the most efficient method. It does not clone strings.
#[derive(Serialize, FromRow)]
pub struct WidgetRecordRef<'a> {
    pub id: i64,
    pub serial: i64,
    pub name: &'a str,
    pub description: &'a str,
}

#[post("/widgetsref")]
pub async fn widgetsref(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::new(
            RowStream::build(
                (pool.as_ref().clone(), "SELECT * FROM widgets LIMIT $1 OFFSET $2 "),
                move |(pool, sql)| {
                    sqlx::query(sql)
                        .bind(params.limit)
                        .bind(params.offset)
                        .fetch(pool)
                }
            ),
            |buf: &mut BytesWriter, row: &PgRow| {
                serde_json::to_writer(
                    buf,
                    &WidgetRecordRef::from_row(row).map_err(ErrorInternalServerError)?,
                )
                .map_err(ErrorInternalServerError)
            },
        ))
}

#[post("/widget_table")]
pub async fn widget_table(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            ByteStream::new(
                RowStream::build(
                    (pool.as_ref().clone(), params),
                    move |(pool, params)| {
                        sqlx::query_as!(
                            WidgetRecord,
                            "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                            params.limit,
                            params.offset
                        )
                            .fetch(pool)
                    }
                ),
                |buf: &mut BytesWriter, rec: &WidgetRecord| {
                    write!(
                        &mut *buf,
                        r#"[{}, {}, "{}", "{}"]"#,
                        rec.id, rec.serial, rec.name, rec.description,
                    )
                    .map_err(ErrorInternalServerError)
                },
            )
            .prefix(r#"{"cols":["id", "serial", "name", "description"],"rows":["#)
            .suffix(r#"]}"#),
        )
}

// This is very inefficient; however, it shows how the json array can
// be constructed using stream combinators.
#[post("/combinators")]
pub async fn combinators(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            stream::once(future::ready({
                let mut b = BytesMut::new();
                b.put_u8(b'[');
                Ok(b.freeze())
            }))
            .chain(
                RowStream::build((pool.as_ref().clone(), params), move |(pool, params)| {
                    sqlx::query_as!(
                        WidgetRecord,
                        "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                        params.limit,
                        params.offset
                    )
                    .fetch(pool)
                })
                .enumerate()
                .map(|(i, item)| {
                    item.map(|row| {
                        let mut writer = BytesWriter(BytesMut::new());
                        if i > 0 {
                            writer.0.put_u8(b',');
                        }
                        serde_json::to_writer(&mut writer, &row).ok();
                        writer.freeze()
                    })
                    .map_err(ErrorInternalServerError)
                }),
            )
            .chain(stream::once(future::ready({
                let mut b = BytesMut::new();
                b.put_u8(b']');
                Ok(b.freeze())
            }))),
        )
}

pub fn service(cfg: &mut web::ServiceConfig) {
    cfg.service(widgets);
    cfg.service(widgets2);
    cfg.service(widgetsref);
    cfg.service(widget_table);
    cfg.service(combinators);
}
