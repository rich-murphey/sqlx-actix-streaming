# sqlx-actix-streaming
Stream [sqlx](https://github.com/launchbadge/sqlx) database query results via an [actix-web](https://actix.rs/) HTTP response.

This example uses sqlx to get a stream of SQL query results. It
streams the results as json in an HTTP response body.  For a very
large response body (megabytes or larger) this can significantly
reduce latency compared to buffering the entire response before
sending. This is the /widgets HTTP method in
[example/src/widgets.rs](example/src/widgets.rs):

````rust
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
        ),
    )
}
````

Here is the same example using fewer macros:
* [sqlx::query_as!()](https://docs.rs/sqlx/0.4.2/sqlx/macro.query_as.html) is a stream of WidgetRecords that borrows
  a database connection and parameters.
* ByteStreamWithParams::new() wraps it with an owned database
  connection and owned parameters, and coverts each record to json.
* [HttpResponse.streaming()](https://docs.rs/actix-web/3.3.2/actix_web/dev/struct.HttpResponseBuilder.html#method.streaming) streams it to the HTTP client.

Note the two closures.  The first closure generates a stream of
WidgetRecords.  The second closure converts an individual WidgetRecord
into json text using serde.  ByteStreamWithParams converts them to a json array.

````rust
#[post("/widgets")]
pub async fn widgets(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            // this is a stream of text Bytes of a JSON array of sqlx records
            ByteStreamWithParams::new(
                pool.as_ref().clone(),
                params,
                move |pool, params| {
                    // this is a a stream of WidgetRecords that borrows pool and params
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
````

To test this, invoke the web server using `cargo run`, and while it
is running, query the HTTP method, for example `curl -s -H 'Content-Type: application/json' -d '{"offset":0,"limit":100}' http://localhost:8080/widgets |jq`. The output is:

````json
[
  {
    "id": 1,
    "serial": 10138,
    "name": "spanner",
    "description": "blue 10 guage joint spanner"
  },
  {
    "id": 2,
    "serial": 39822,
    "name": "flexarm",
    "description": "red flexible support arm"
  },
  {
    "id": 3,
    "serial": 52839,
    "name": "bearing",
    "description": "steel bearing for articulating joints"
  }
]
````

See [example/src/widgets.rs](example/src/widgets.rs) for more
details. It also shows variations in json format (array vs object) and
in record type (struct vs tuple).

## Minimum Supported Rust Version

Requires Rust **1.45** or newer.
