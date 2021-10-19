use barrel::{Table,types, Migration};
use barrel::backend::Pg;

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
    
    migrate(&pool).await?;
     
    Ok(())
}

pub async fn migrate(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(),sqlx::Error> {
    let mut m = Migration::new();

    m.create_table("posts", |t| {
        t.add_column("id", types::primary());
        t.add_column("post", types::varchar(255));
        t.add_column("url", types::varchar(255).indexed(true));
       
    });
    
    let res:sqlx::postgres::PgQueryResult =  sqlx::query(&m.make::<Pg>()).execute(pool).await?;
    println!("{:?}",res);

    let mut m = Migration::new();
    // A new table is automatically created with an "id" primary key
    // To disable that call `without_id` on the return of `create_table`
    m.create_table("users", |t: &mut Table| {
        t.add_column("name", types::varchar(255)); // Default name is "Anonymous"
        t.add_column("description", types::text().nullable(true)); // Can be null
        t.add_column("age", types::integer());
        t.add_column("posts", types::foreign("posts", vec!["id"]));
        t.add_column("owns_plushy_sharks", types::boolean());
    });

    // CREATE TABLE "users" (
    //     "name" VARCHAR(255) NOT NULL, 
    //     "description" TEXT, 
    //     "age" INTEGER NOT NULL, 
    //     "posts" INTEGER REFERENCES 
    //     "posts"(id,url) NOT NULL, 
    //     "owns_plushy_sharks" BOOLEAN NOT NULL);
    //println!("{}", m.make::<Pg>());

    let res:sqlx::postgres::PgQueryResult = sqlx::query(&m.make::<Pg>()).execute(pool).await?;
    println!("{:?}",res);
    Ok(())
}
