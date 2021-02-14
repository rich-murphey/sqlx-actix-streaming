use actix_web::*;
use anyhow::Context;
use log::*;
use std::env;

mod widgets;

type Db = sqlx::postgres::Postgres;
type DbPool = sqlx::Pool<Db>;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let pool = DbPool::connect(&env::var("DATABASE_URL").context("DATABASE_URL")?).await?;
    let addr = env::var("SOCKETADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    info!("this web server is listening at http://{}", &addr);
    HttpServer::new(move || {
        actix_web::App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .configure(widgets::service)
            .default_service(web::route().to(HttpResponse::NotFound))
    })
    .bind(&addr)
    .context(addr)?
    .run()
    .await
    .context("While running actix web server")?;
    Ok(())
}
