# sqlx-actix-streaming
Stream sqlx database query results to an actix HTTP (streaming) response body.

In the example below, a sqlx query response is streamed as an HTTP
response body. For a very large response body (megabytes or larger)
this can significantly reduce latency compared to buffering the entire
response before sending.

In the /widgets HTTP API method below:

* sqlx::query_as!().fetch() is a borrowed stream of WidgetRecords.
* RowStream::gen() converts it to an owned stream of WidgetRecords.
* ByteStream::json_array() converts it to a text stream of a json array.
* HttpResponse.streaming() streams it to the client.

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
            ByteStream::json_array(
                RowStream::gen(
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
                )
            )
        )
}
````

The output of `curl -s -H 'Content-Type: application/json' -d '{"offset":0,"limit":100}' http://localhost:8080/widgets |jq` is:

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

See example/src/widgets.rs for more details, as well as, other
variations in json array or object responses and kinds of records.
