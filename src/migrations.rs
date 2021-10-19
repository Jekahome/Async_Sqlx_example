
use sqlx::postgres::PgPoolOptions;
use sqlx_example::settings;
use std::time::Duration;


#[tokio::main]
async fn main() -> Result<(),sqlx::Error> {
    // Create a connection pool
    //  for MySQL, use sqlx::mysql::MySqlPoolOptions::new()
    //  for SQLite, use SqlitePoolOptions::new(), SqliteConnection::connect("sqlite::memory:") 
    //  etc.

    let config:String = settings::config().expect("Error parse config");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .max_lifetime(Duration::from_secs(30 * 60))
        .connect(&config)
        .await?;
    
    // migrate (создастся таблица _sqlx_migrations)
        migrate(&pool).await?;

    Ok(())
}

pub async fn migrate(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
    // used macro  
    // sqlx::migrate!("./migrations").run(pool).await

    // or
   
   use sqlx::migrate::Migrator;
   let m = Migrator::new(std::path::Path::new("./migrations")).await?;
   m.run(pool).await
}