use actix_web::*;
use serde::*;
use sqlx::{postgres::*, prelude::*};
use sqlx_actix_streaming::*;

#[derive(Serialize, FromRow)]
pub struct WidgetRec {
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
pub struct WidgetRec2<'a> {
    pub id: i64,
    pub serial: i64,
    pub name: &'a str,
    pub description: &'a str,
}
#[post("/widgets5")]
pub async fn widgets5(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse
{
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            ByteStream::pin(
                RowStream::pin(
                    pool.as_ref().clone(),
                    |pool| {
                        sqlx::query_as!(
                            WidgetRec,
                            "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                            params.limit,
                            params.offset
                        )
                            .fetch(pool)
                    }
                ),
                |buf: &mut BytesWriter, record| {
                    serde_json::to_writer(buf, record)
                        .map_err(error::ErrorInternalServerError)
                },
            )
        )
}

#[post("/widgets4")]
pub async fn widgets4(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::pin(
            RowWStmtStream::pin(
                pool.as_ref().clone(),
                "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                |pool, sql| {
                    sqlx::query(sql)
                        .bind(params.limit)
                        .bind(params.offset)
                        .fetch(pool)
                },
            ),
            |buf: &mut BytesWriter, row| {
                serde_json::to_writer(
                    buf,
                    &WidgetRec2::from_row(row).map_err(error::ErrorInternalServerError)?,
                )
                .map_err(error::ErrorInternalServerError)
            },
        ))
}

#[post("/widgets")]
pub async fn widgets(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(query_as_byte_stream!(
            WidgetRec,
            pool.as_ref().clone(),
            "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
            |buf: &mut BytesWriter, rec: &WidgetRec| {
                serde_json::to_writer(buf, rec).map_err(error::ErrorInternalServerError)
            },
            params.limit,
            params.offset
        ))
}

// note: no macros used here.
#[post("/widgets2")]
pub async fn widgets2(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::json_array(RowStream::pin(
            pool.as_ref().clone(),
            |pool| {
                sqlx::query_as!(
                    WidgetRec,
                    "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                    params.limit,
                    params.offset
                )
                .fetch(pool)
            },
        )))
}
#[post("/widgets3")]
pub async fn widgets3(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::json_array(RowWStmtStream::pin(
            pool.as_ref().clone(),
            "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
            |pool, sql| {
                sqlx::query_as::<Postgres, WidgetRec>(sql)
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch(pool)
            },
        )))
}

#[post("/widget_table")]
pub async fn widget_table(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            query_as_byte_stream!(
                WidgetRec,
                pool.as_ref().clone(),
                "SELECT * FROM widgets LIMIT $1 OFFSET $2 ",
                |buf: &mut BytesWriter, rec: &WidgetRec| {
                    write!(
                        &mut *buf,
                        r#"[{}, {}, "{}", "{}"]"#,
                        rec.id, rec.serial, rec.name, rec.description,
                    )
                    .map_err(error::ErrorInternalServerError)
                },
                params.limit,
                params.offset
            )
            .prefix(r#"{"cols":["id", "serial", "name", "description"],"rows":["#)
            .suffix(r#"]}"#),
        )
}

#[post("/widget_table2")]
pub async fn widget_table2(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let mut prefix = r#"{"cols":["id", "serial", "name", "description"],"params":"#.to_string();
    prefix.push_str(&serde_json::to_string(&params).unwrap());
    prefix.push_str(r#","rows":["#);
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            query_byte_stream!(
                pool.as_ref().clone(),
                "SELECT * FROM widgets LIMIT $1 OFFSET $2 ".to_string(),
                |buf: &mut BytesWriter, row| {
                    serde_json::to_writer(
                        buf,
                        &WidgetRec2::from_row(row).map_err(error::ErrorInternalServerError)?,
                    )
                    .map_err(error::ErrorInternalServerError)
                },
                params.limit,
                params.offset
            )
            .prefix(prefix)
            .suffix(r#"]}"#),
        )
}

#[derive(Serialize, FromRow)]
pub struct WidgetTuple<'a>(i64, i64, &'a str, &'a str);

#[post("/widgetrows")]
pub async fn widgetrows(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            query_byte_stream!(
                pool.as_ref().clone(),
                "SELECT * FROM widgets LIMIT $1 OFFSET $2 ".to_string(),
                |buf: &mut BytesWriter, row| {
                    serde_json::to_writer(
                        buf,
                        &WidgetTuple::from_row(row).map_err(error::ErrorInternalServerError)?,
                    )
                    .map_err(error::ErrorInternalServerError)
                },
                params.limit,
                params.offset
            )
            .prefix(r#"{"cols":["id", "serial", "name", "description"],"rows":["#)
            .suffix(r#"]}"#),
        )
}

#[derive(Serialize, FromRow)]
pub struct WidgetTuple2(i64, i64, String, String);

#[post("/widgetrows2")]
pub async fn widgetrows2(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let sql = "SELECT * FROM widgets LIMIT $1 OFFSET $2 ".to_string();
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            query_as_byte_stream!(
                WidgetTuple2,
                pool.as_ref().clone(),
                sql,
                |buf: &mut BytesWriter, rec: &WidgetTuple2| {
                    serde_json::to_writer(buf, rec).map_err(error::ErrorInternalServerError)
                },
                params.limit,
                params.offset
            )
            .prefix(r#"{"cols":["id", "serial", "name", "description"],"rows":["#)
            .suffix(r#"]}"#),
        )
}

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
