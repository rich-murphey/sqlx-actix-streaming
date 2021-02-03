# sqlx-actix-streaming
Stream [sqlx](https://github.com/launchbadge/sqlx) database query results via an [actix-web](https://actix.rs/) HTTP response.

In the example below, a SQL query result is streamed via an HTTP
response body. For a very large response body (megabytes or larger)
this can significantly reduce latency compared to buffering the entire
response before sending.

In the /widgets HTTP method from [example/src/widgets.rs](example/src/widgets.rs) below:

* [sqlx::query_as!().fetch()](https://docs.rs/sqlx/0.4.2/sqlx/macro.query_as.html) is a stream of WidgetRecords that borrows
  a database connection.
* RowStream::make() wraps it with an owned database connection.
* ByteStream::make() converts it to a json text stream.
* [HttpResponse.streaming()](https://docs.rs/actix-web/3.3.2/actix_web/dev/struct.HttpResponseBuilder.html#method.streaming) streams it to the client.

Note the two closures.  The first closure generates a stream of
WidgetRecords.  The second closure converts an individual
WidgetRecord into json text using serde.  ByteStream wraps them in json
array syntax, using '[', ',' and ']' by default.

````rust
#[derive(Serialize, FromRow)]
pub struct WidgetRecord {
    pub id: i64,
    pub serial: i64,
    pub name: String,
    pub description: String,
}
#[post("/widgets")]
pub async fn widgets(
    web::Json(params): web::Json<WidgetParams>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(
            // this is a stream of a JSON array of WidgetRecords
            ByteStream::make(
                // this is a stream of WidgetRecords that owns Pool
                RowStream::make(pool.as_ref(), |pool| {
                    // this is a stream of WidgetRecords that borrows Pool
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
                    serde_json::to_writer(buf, record).map_err(error::ErrorInternalServerError)
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
