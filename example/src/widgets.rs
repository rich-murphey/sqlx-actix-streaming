use actix_web::{error::ErrorInternalServerError, *};
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

// NOTE: this is the most efficient method. It does not clone strings.
#[derive(Serialize, FromRow)]
pub struct WidgetRecord2<'a> {
    pub id: i64,
    pub serial: i64,
    pub name: &'a str,
    pub description: &'a str,
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
        ),
    )
}

#[post("/widgets2")]
pub async fn widgets2(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    json_response_alt!(
        WidgetRecord,
        pool.as_ref(),
        "SELECT * FROM widgets LIMIT $1 OFFSET $2 ".to_owned(),
        params.limit,
        params.offset
    )
}

#[post("/widgets3")]
pub async fn widgets3(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            // a stream of Bytes containing an JSON text array of sqlx records
            ByteStreamWithParams::new(
                pool.as_ref().clone(),
                params,
                move |pool, params| {
                    // this is a a stream of WidgetRecords that borrows Pool
                    sqlx::query_as!(
                        WidgetRecord,
                        "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                        params.limit,
                        params.offset
                    )
                    .fetch(pool)
                },
                |buf: &mut BytesWriter, record: &WidgetRecord| {
                    // this writes a WidgetRecords as JSON text to the output buffer
                    serde_json::to_writer(buf, record).map_err(ErrorInternalServerError)
                },
            ),
        )
}

#[post("/widgets4")]
pub async fn widgets4(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            // a stream of Bytes containing an JSON text array of sqlx records
            ByteStream::new(
                pool.as_ref(),
                move |pool| {
                    // this is a a stream of WidgetRecords that borrows Pool
                    sqlx::query_as!(
                        WidgetRecord,
                        "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                        params.limit,
                        params.offset
                    )
                    .fetch(pool)
                },
                |buf: &mut BytesWriter, record: &WidgetRecord| {
                    // this writes a WidgetRecords as JSON text to the output buffer
                    serde_json::to_writer(buf, record).map_err(ErrorInternalServerError)
                },
            ),
        )
}

#[post("/widgets5")]
pub async fn widgets5(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStreamDyn::new(
            pool.as_ref(),
            "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
            move |pool, sql| {
                sqlx::query(sql)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch(pool)
            },
            |buf: &mut BytesWriter, row: &PgRow| {
                serde_json::to_writer(
                    buf,
                    &WidgetRecord2::from_row(row).map_err(ErrorInternalServerError)?,
                )
                .map_err(ErrorInternalServerError)
            },
        ))
}

// #[post("/widgets6")]
// pub async fn widgets6(
//     web::Json(params): web::Json<WidgetParams>,
//     pool: web::Data<PgPool>,
// ) -> HttpResponse {
//     HttpResponse::Ok()
//         .content_type("application/json")
//         .streaming(query_as_byte_stream!(
//             WidgetRecord,
//             pool.as_ref(),
//             "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
//             |buf: &mut BytesWriter, rec: &WidgetRecord| {
//                 serde_json::to_writer(buf, rec).map_err(ErrorInternalServerError)
//             },
//             params.limit,
//             params.offset
//         ))
// }

// #[post("/widget_table")]
// pub async fn widget_table(
//     web::Json(params): web::Json<WidgetParams>,
//     pool: web::Data<PgPool>,
// ) -> HttpResponse {
//     HttpResponse::Ok()
//         .content_type("application/json")
//         .streaming(
//             query_as_byte_stream!(
//                 WidgetRecord,
//                 pool.as_ref(),
//                 "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
//                 |buf: &mut BytesWriter, rec: &WidgetRecord| {
//                     write!(
//                         &mut *buf,
//                         r#"[{}, {}, "{}", "{}"]"#,
//                         rec.id, rec.serial, rec.name, rec.description,
//                     )
//                     .map_err(ErrorInternalServerError)
//                 },
//                 params.limit,
//                 params.offset
//             )
//             .prefix(r#"{"cols":["id", "serial", "name", "description"],"rows":["#)
//             .suffix(r#"]}"#),
//         )
// }

// #[post("/widget_table2")]
// pub async fn widget_table2(
//     web::Json(params): web::Json<WidgetParams>,
//     pool: web::Data<PgPool>,
// ) -> HttpResponse {
//     let mut prefix = r#"{"cols":["id", "serial", "name", "description"],"params":"#.to_string();
//     prefix.push_str(&serde_json::to_string(&params).unwrap());
//     prefix.push_str(r#","rows":["#);
//     HttpResponse::Ok()
//         .content_type("application/json")
//         .streaming(
//             query_byte_stream!(
//                 pool.as_ref(),
//                 "SELECT * FROM widgets LIMIT $1 OFFSET $2 ".to_string(),
//                 |buf: &mut BytesWriter, row| {
//                     serde_json::to_writer(
//                         buf,
//                         &WidgetRecord2::from_row(row).map_err(ErrorInternalServerError)?,
//                     )
//                     .map_err(ErrorInternalServerError)
//                 },
//                 params.limit,
//                 params.offset
//             )
//             .prefix(prefix)
//             .suffix(r#"]}"#),
//         )
// }

// #[derive(Serialize, FromRow)]
// pub struct WidgetTuple<'a>(i64, i64, &'a str, &'a str);

// #[post("/widgetrows")]
// pub async fn widgetrows(
//     web::Json(params): web::Json<WidgetParams>,
//     pool: web::Data<PgPool>,
// ) -> HttpResponse {
//     HttpResponse::Ok()
//         .content_type("application/json")
//         .streaming(
//             query_byte_stream!(
//                 pool.as_ref(),
//                 "SELECT * FROM widgets LIMIT $1 OFFSET $2 ".to_string(),
//                 |buf: &mut BytesWriter, row| {
//                     serde_json::to_writer(
//                         buf,
//                         &WidgetTuple::from_row(row).map_err(ErrorInternalServerError)?,
//                     )
//                     .map_err(ErrorInternalServerError)
//                 },
//                 params.limit,
//                 params.offset
//             )
//             .prefix(r#"{"cols":["id", "serial", "name", "description"],"rows":["#)
//             .suffix(r#"]}"#),
//         )
// }

#[derive(Serialize, FromRow)]
pub struct WidgetTuple2(i64, i64, String, String);

// #[post("/widgetrows2")]
// pub async fn widgetrows2(
//     web::Json(params): web::Json<WidgetParams>,
//     pool: web::Data<PgPool>,
// ) -> HttpResponse {
//     let sql = "SELECT * FROM widgets LIMIT $1 OFFSET $2 ".to_string();
//     HttpResponse::Ok()
//         .content_type("application/json")
//         .streaming(
//             query_as_byte_stream!(
//                 WidgetTuple2,
//                 pool.as_ref(),
//                 sql,
//                 |buf: &mut BytesWriter, rec: &WidgetTuple2| {
//                     serde_json::to_writer(buf, rec).map_err(ErrorInternalServerError)
//                 },
//                 params.limit,
//                 params.offset
//             )
//             .prefix(r#"{"cols":["id", "serial", "name", "description"],"rows":["#)
//             .suffix(r#"]}"#),
//         )
// }

use futures::stream::StreamExt;
#[post("/widgetrows3")]
pub async fn widgetrows3<'a>(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let sql = "SELECT * FROM widgets LIMIT $1 OFFSET $2 ";
    HttpResponse::Ok().content_type("application/json").json(
        sqlx::query(&sql)
            .bind(params.limit)
            .bind(params.offset)
            .fetch(pool.as_ref())
            .map(|r| WidgetTuple2::from_row(&r.unwrap()).unwrap())
            .collect::<Vec<WidgetTuple2>>()
            .await,
    )
}
pub fn service(cfg: &mut web::ServiceConfig) {
    cfg.service(widgets);
    cfg.service(widgets2);
    cfg.service(widgets3);
    cfg.service(widgets4);
    cfg.service(widgets5);
    // cfg.service(widgets6);
    // cfg.service(widget_table);
    // cfg.service(widget_table2);
    // cfg.service(widgetrows);
    // cfg.service(widgetrows2);
    cfg.service(widgetrows3);
}
