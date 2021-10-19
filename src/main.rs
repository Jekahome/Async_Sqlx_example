
use sqlx::Connection;
use sqlx::Executor;
use sqlx::Statement;
use sqlx::postgres::PgPoolOptions;
use sqlx::postgres::PgConnectOptions;
use sqlx_example::settings;
use std::convert::TryFrom;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(),anyhow::Error> {
    // Create a connection pool
    //  for MySQL, use sqlx::mysql::MySqlPoolOptions::new()
    //  for SQLite, use SqlitePoolOptions::new(), SqliteConnection::connect("sqlite::memory:") 
    //  etc.

    // conn через PgPoolOptions или sqlx::pool::PoolOptions::<sqlx::postgres::Postgres>::new()
    let config:String = settings::config().expect("Error parse config");
    let mut pool:sqlx::Pool<sqlx::Postgres> = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .max_lifetime(Duration::from_secs(30 * 60))
        .connect(&config)
        .await
        .map_err(|err| self::Error::ConnectingToDatabase(err.to_string()))?;
    
    /*
    // conn через PgConnectOptions
    use sqlx::ConnectOptions;
    let mut pool:sqlx::PgConnection = settings::config_2().expect("Error parse config").connect().await?;
    */

    // conn через sqlx::PgConnection  
    // let mut pool:sqlx::PgConnection = <sqlx::postgres::Postgres as sqlx::Database>::Connection::connect(&settings::config().expect("Error parse config")).await?;

    // query ----------------------------------------------------------------------------------------------------------------------------------
     query_example(&pool).await?;

    // error ----------------------------------------------------------------------------------------------------------------------------------
     error_example(&pool).await?;

    // transaction ----------------------------------------------------------------------------------------------------------------------------
     transaction_example(&pool).await?;

    // listener -------------------------------------------------------------------------------------------------------------------------------
    test_listener_cleanup().await?;
    
    // COPY IN Postgres
    copy_in_example().await?;

   
    tokio::time::sleep(Duration::from_secs(2)).await;
    pool.close();

    Ok(())
}

pub async fn new<DB>() -> anyhow::Result<DB::Connection>
where
    DB: sqlx::Database,
{
    
    Ok(DB::Connection::connect(&settings::config().expect("Error parse config")).await?)
}


async fn copy_in_example() -> anyhow::Result<()> {
    let mut conn = new::<sqlx::postgres::Postgres>().await?;

    let mut sink/*:sqlx::postgres::copy::PgCopyIn<&mut sqlx::PgConnection>*/ = conn.copy_in_raw("COPY todo FROM STDIN WITH DELIMITER E'\t' CSV").await?;
  
    let reader =  String::from("51\tjim\t2021-10-18 23:40:52.319 +0300\t2021-10-18 23:40:52.319 +0300\t1
    52\tjim\t2021-10-18 23:40:52.319 +0300\t2021-10-18 23:40:52.319 +0300\t1").into_bytes();
    sink.send(reader).await.unwrap();
  
    let rows = sink.finish().await.unwrap();
    println!("{}",rows);

    Ok(())
}

async fn test_listener_cleanup() -> anyhow::Result<()> {
    //https://github.com/launchbadge/sqlx/blob/be189bd11e6bdd14c45c70bdad477e780a82b050/tests/postgres/postgres.rs#L898
    use sqlx::postgres::PgListener;
    use tokio::time::timeout;

    let pool:sqlx::Pool<sqlx::postgres::Postgres> = sqlx::pool::PoolOptions::<sqlx::postgres::Postgres>::new()
        .min_connections(1)
        .max_connections(1)
        .test_before_acquire(true)
        .connect(&settings::config().expect("Error parse config"))
        .await?;

    let mut listener = PgListener::connect_with(&pool).await?;
    listener.listen("test_channel").await?;


    // Проверяет наличие уведомления на тестовом канале
    async fn try_recv(listener: &mut PgListener) -> anyhow::Result<bool> {
        match timeout(Duration::from_millis(100), listener.recv()).await {
            Ok(res) => {
                res?;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    // Создайте соединение для отправки уведомлений
    let mut notify_conn = new::<sqlx::postgres::Postgres>().await?;

    // Убедитесь, что уведомление не получено, прежде чем оно будет отправлено
    assert!(!try_recv(&mut listener).await?, "Notification not sent");

    // Уведомление о чеке отправлено и получено
    notify_conn.execute("NOTIFY test_channel").await?;

    assert!(
        try_recv(&mut listener).await?,
        "Notification sent and received"
    );
    assert!(
        !try_recv(&mut listener).await?,
        "Notification is not duplicated"
    );

    Ok(())
}

async fn transaction_example(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(),anyhow::Error>{
    use chrono::{DateTime,Utc,Local};
  
    let mut transaction:sqlx::Transaction<sqlx::Postgres> = pool.begin().await?;
   
    let utc: DateTime<Local> = Utc::now().with_timezone(&Local);
    //----------------------------------------------------------------------------------------------------------------------------------------------------------

    let stmt = sqlx::query("INSERT INTO todo(name, created_at,checked_date,checked) VALUES ($1::VARCHAR, $2::TIMESTAMPTZ, $3::TIMESTAMPTZ, $4::BOOL);")
    .bind("bla name").bind(utc).bind(utc).bind(true);
    let res:sqlx::postgres::PgQueryResult = transaction.execute(stmt).await?;
    println!("rows_affected:{:?}",res.rows_affected());

    // или другим способом выполнить запрос
    let stmt = sqlx::query("INSERT INTO todo(name, created_at,checked_date,checked) VALUES ($1::VARCHAR, $2::TIMESTAMPTZ, $3::TIMESTAMPTZ, $4::BOOL);");
    let res:sqlx::postgres::PgQueryResult = stmt.bind("bla 2 name").bind(utc).bind(utc).bind(true).execute(&mut transaction).await?;
    println!("rows_affected:{:?}",res.rows_affected());


   // подготовка запроса и использование его клона -------------------------------------------------------------------------------------------------------------
   let stmt:sqlx::postgres::PgStatement =  transaction.prepare_with(
        "INSERT INTO todo(name, created_at,checked_date,checked) VALUES ($1::VARCHAR, $2::TIMESTAMPTZ, $3::TIMESTAMPTZ, $4::BOOL) RETURNING id;",&[
        sqlx::postgres::PgTypeInfo::with_name("VARCHAR"),
        sqlx::postgres::PgTypeInfo::with_name("TIMESTAMPTZ"),
        sqlx::postgres::PgTypeInfo::with_name("TIMESTAMPTZ"),
        sqlx::postgres::PgTypeInfo::with_name("BOOL")]).await?;

    let stmt_clone = stmt.clone();
    // --------------------------------------
    // query требует преобразования результатов из PgRow
    {
         use sqlx::Row;// Для метода try_get
         let row:sqlx::postgres::PgRow = stmt_clone.query().bind("1 bla name").bind(utc).bind(utc).bind(true).fetch_one(&mut transaction).await?;
         println!("id:{:?} ",row.try_get::<i32,_>(0)?);
         
    }  
    // --------------------------------------
    // query_as_with сразу преобразует результат в тип реализующий PgRow
    use sqlx::Arguments;// для метода add
    let mut arg = sqlx::postgres::PgArguments::default();
    arg.add("2 bla name");
    arg.add(utc);
    arg.add(utc);
    arg.add(true);

    {   
        // Id реализует PgRow
        let id:Id = stmt_clone.query_as_with::<Id, sqlx::postgres::PgArguments>(arg).fetch_one(&mut transaction).await?;
        println!("id:{:?} ",id);
    } 
    
    // --------------------------------------
    {
       // Id реализует PgRow
       let id:Id = stmt_clone.query_as::<Id>().bind("3 bla name").bind(utc).bind(utc).bind(true).fetch_one(&mut transaction).await?;
        println!("id:{:?} ",id);
    } 
   

    transaction.commit().await?;
    // transaction.rollback().await?;

    Ok(())
}

async fn error_example(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(),anyhow::Error>{
   use  sqlx::error::DatabaseError;

   let res:std::result::Result<sqlx::postgres::PgRow, sqlx::Error> = sqlx::query("SELECT -").fetch_one(pool).await;
   match res {
       Ok(_) =>{},
       Err(err) =>{
       
        match err {
            sqlx::Error::Database(pg_db_error) => {
               
                let err:sqlx::postgres::PgDatabaseError =  *(pg_db_error.into_error().downcast::<sqlx::postgres::PgDatabaseError>().unwrap());
                println!("Error:{:?}",err.file()); 
            },
            _ => {}
        }
       }
   }
    Ok(())
}

// sqlx::Postgres implementation sqlx::Database
// sqlx::postgres::PgRow implementation sqlx::Row<Database = sqlx::Postgres>
async fn query_example(pool: /*&sqlx::PgConnection */ &sqlx::Pool<sqlx::Postgres>) -> Result<(),anyhow::Error>{
    // (use a question mark `?` instead of `$1` for MySQL)
     
    use futures::{TryStreamExt};
    use sqlx::Row;
     

    let mut rows = sqlx::query("SELECT id,name,created_at,checked FROM todo WHERE id = $1::INT4 AND id > $2::INT4")
        .bind(1)
        .bind(0)
        .fetch(pool);
    while let Some(row) = rows.try_next().await? {
        // map the row into a user-defined domain type
        let id: i32 = row.try_get("id")?;// or .try_get(0)
        let name: &str = row.try_get("name")?;// or .try_get(1)
        println!("id:{} ,name:{}",id,name);
    }
    
    // идеоматичный запрос для доменных типов, помочь сопоставить типы
    let mut stream = sqlx::query("SELECT id,name,created_at,checked FROM todo WHERE id = $1::INT4 AND id > $2::INT4")
    .bind(1)
    .bind(0)
    .map(|row: sqlx::postgres::PgRow| {
        let id: i32 = row.try_get("id").map_err(|err:sqlx::Error| self::Error::Internal(err.to_string())).unwrap();
        let name: &str = row.try_get("name").map_err(|err:sqlx::Error| self::Error::Internal(err.to_string())).unwrap();
        println!("id:{} ,name:{}",id,name);
    })
    .fetch(pool);
    stream.try_next().await?;


   // идеоматичный запрос для доменных типов
   // query_as

    let mut user:User = sqlx::query_as::<_, User>("SELECT id,name,created_at,'Two' as variant FROM todo WHERE id = $1::INT4")
        .bind(Id(1))
        .fetch_one(pool).await?;
    println!("user {:?}",user);

    let variant:Variant = sqlx::query_as::<_, Variant>("SELECT 'two' as variant FROM todo WHERE id = $1::INT4")
        .bind(Id(1))
        .fetch_one(pool).await?;
     println!("variant {:?}",variant);

     let mut id:Id = sqlx::query_as::<_, Id>("SELECT 1")
        .fetch_one(pool).await?;
     println!("id {:?}",id);


    // Способ трейтов
    use sqlx::Executor;// trait impl for sqlx::pool::Pool or sqlx::pool::PoolConnection or sqlx::connection::Connection
    use sqlx::Execute;// trait impl for &str or  sqlx::query::Query
   
    // аргумент &str
    let variant:Variant = pool.fetch_one("SELECT 'two' as variant FROM todo WHERE id = 1").await?.try_get("variant")?;
    println!("variant {:?}",variant);
    
    // вариант через задницу
    use sqlx::Statement;
    let variant:Variant = pool.prepare_with("SELECT 'two' as variant FROM todo WHERE id = $1",&[ sqlx::postgres::PgTypeInfo::with_name("INT4")])
    .await?.query().bind(Id(1)).fetch_one(pool).await?.try_get("variant")?;
    println!("variant {:?}",variant);

    // аргумент sqlx::query::Query
    let variant:Variant =  pool.fetch_one(sqlx::query("SELECT 'two' as variant FROM todo WHERE id = $1::INT4").bind(Id(1))).await?.try_get("variant")?;
    println!("variant {:?}",variant);

 //-----------------------------------------------------------------------------------------------------------------------------------------------------------
    // sqlx::query_as, sqlx::query_as_with, sqlx::query_scalar  Function
    // Сделайте SQL-запрос, который сопоставлен с конкретным типом, используя FromRow.

    let variant:Variant = sqlx::query_as("SELECT 'two' as variant FROM todo WHERE id = $1::INT4").bind(Id(1)).fetch_one(pool).await?;
    println!("variant:{:?} ",variant);
 

    let (todo_id,): (i32,) = sqlx::query_as(
        "
            INSERT INTO todo (name)
            VALUES ($1)
            RETURNING id
        ",
    )
    .bind("bla").fetch_one(pool).await?;
 
    // query_as_with
    use sqlx::Arguments;
    let mut arg = sqlx::postgres::PgArguments::default();
    arg.add(Id(1));// impl Encode,Type

    sqlx::query_as_with::<sqlx::Postgres,Variant, sqlx::postgres::PgArguments>("SELECT 'two' as variant FROM todo WHERE id = $1::INT4", arg  );

    // sqlx::query_scalar
    let variant:Variant = sqlx::query_scalar("SELECT 'two' as variant FROM todo WHERE id = $1::INT4").bind(Id(1)).fetch_one(pool).await?;
    println!("variant:{:?} ",variant);


 //-----------------------------------------------------------------------------------------------------------------------------------------------------------
  
 // https://docs.rs/sqlx/0.5.9/sqlx/query/struct.Query.html#method.try_map
      
/*
Методы sqlx::query::Query:
    bind - привязка типов к аргументам
    execute  - выполнить запрос и верните количество затронутых строк.
    execute_many - Выполните несколько запросов и верните количество затронутых строк в потоке.
    fetch
    fetch_all
    fetch_many
    fetch_one
    fetch_optional
    try_map, map - Сопоставьте каждую строку результата с другим типом.
    persistent
*/
    // conn.execute(sqlx::query("DELETE FROM table")).await?; 
    // or
    // sqlx::query("DELETE FROM table").execute(&pool).await?;

   // execute
   let rows =  sqlx::query("INSERT INTO todo (name,checked) VALUES ($1::VARCHAR,$2::BOOL);").bind("pups").bind(true).execute(pool).await?;
   println!("count:{:?} ",rows.rows_affected());

   // execute_many
   let mut rows = sqlx::query("INSERT INTO todo (name,checked) VALUES ($1::VARCHAR,$2::BOOL);").bind("pups").bind(true).execute_many(pool).await;
   while let Some(row) = rows.try_next().await? {
    println!("count:{} ",row.rows_affected());
   }

    // fetch
    let mut rows = sqlx::query("SELECT 'two' as variant FROM todo WHERE id > $1::INT4").bind(0).fetch(pool);
    while let Some(row) = rows.try_next().await? {
        // row:sqlx::postgres::PgRow
        println!("count:{:?} ",row.try_get::<Variant,_>("variant")?);
    }

    // fetch_all
    let mut rows:Vec<sqlx::postgres::PgRow> = sqlx::query("SELECT 'two' as variant FROM todo WHERE id > $1::INT4").bind(0).fetch_all(pool).await?;
    for row in rows.iter(){
    // row:sqlx::postgres::PgRow
        println!("count:{:?} ",row.try_get::<Variant,_>("variant")?);
    }
 
    // fetch_one
    let mut row:sqlx::postgres::PgRow = sqlx::query("SELECT 'two' as variant FROM todo WHERE id = $1::INT4").bind(Id(1)).fetch_one(pool).await?;
    println!("count:{:?} ",row.try_get::<Variant,_>("variant")?);


   // try_map Сопоставьте каждую строку результата с другим типом.
   let value:Variant = sqlx::query("SELECT 'two' as variant FROM todo WHERE id = $1::INT4")
   .bind(Id(1))
   .try_map(|row: sqlx::postgres::PgRow | row.try_get::<Variant, _>(0))
   .fetch_one(pool)
   .await?;
    println!("value {:?}",value);



 //----------------------------------------------------------------------------------------------------------------------------------------------------
/*  
    // Методы  sqlx::postgres::PgRow
    columns
    try_get_raw

    column
    get
    get_unchecked
    is_empty
    len
    try_column
    try_get
    try_get_unchecked
*/

    let mut rows = sqlx::query("SELECT 'two' as variant FROM todo WHERE id = $1::INT4").bind(Id(1)).fetch(pool);
    while let Some(row) = rows.try_next().await? {
        // row:sqlx::postgres::PgRow
        
        // Two
        println!("try_get:{:?} ",row.try_get::<Variant,_>("variant")?);
        // Two
        println!("get:{:?} ",row.get::<Variant,_>("variant"));
        // [PgColumn { ordinal: 0, name: variant, type_info: PgTypeInfo(Text), relation_id: None, relation_attribute_no: None }]
        println!("columns:{:?} ",row.columns());
        // PgColumn { ordinal: 0, name: variant, type_info: PgTypeInfo(Text), relation_id: None, relation_attribute_no: None }
        println!("column:{:?} ",row.column("variant"));
        // PgColumn { ordinal: 0, name: variant, type_info: PgTypeInfo(Text), relation_id: None, relation_attribute_no: None }
        println!("try_column:{:?} ",row.try_column("variant")?);
    }

    Ok(())
}
 
use my_type_safety::{User,Variant,Id};
mod my_type_safety{
    use sqlx::{Row,FromRow,Database};
    use sqlx::decode::Decode;
    use sqlx::encode::Encode;
    use sqlx::types::Type;
    use sqlx::database::HasValueRef;
    use std::error::Error;
    use chrono::{DateTime,Utc,Local};
    use core::fmt::{Debug,Display};
 
     // https://docs.rs/sqlx/0.5.9/sqlx/types/trait.Type.html
     // https://docs.rs/sqlx/0.5.9/sqlx/trait.Decode.html
     // https://docs.rs/sqlx/0.5.9/sqlx/derive.Decode.html
     // https://docs.rs/sqlx/0.5.9/sqlx/derive.Encode.html
     // https://docs.rs/sqlx/0.5.9/sqlx/trait.FromRow.html
     // https://docs.rs/sqlx/0.5.9/sqlx/postgres/struct.PgTypeInfo.html

    // sqlx::FromRow - нужен для преобразования результата из запроса в обьект, (возможно реализовать самостоятельно или через derive)
    // Decode - нужен для парсинга результата строки ,(возможно реализовать самостоятельно или через derive)
    // sqlx::Type - нужен для сопоставления с типом данных, (возможно реализовать самостоятельно или через derive)
    // Encode - нужен для биндинга обьекта в SQL, (возможно реализовать самостоятельно или через derive)
    // #[sqlx(type_name = "mood", rename_all = "lowercase")] - нужен для определения типа данных по имени поля в таблице

    #[derive(Decode,Encode,sqlx::FromRow,Debug,Default)]
    pub struct Id(pub i32);

    impl sqlx::Type<sqlx::Postgres> for Id {
        fn type_info() -> sqlx::postgres::PgTypeInfo {
            sqlx::postgres::PgTypeInfo::with_name("INT4")//типы из sqlx::postgres::PgTypeInfo::INT4
        }
        fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
            *ty == Self::type_info()
        }
    }

    #[derive(Debug)]
     pub struct User { 
        pub name: String, 
        pub id: Id ,
        pub created_at: DateTime<Local>,
       // #[sqlx(rename = "name")]
        pub variant:Variant
     }
     impl std::default::Default for User{
        fn default() -> Self{
            let utc: DateTime<Local> = Utc::now().with_timezone(&Local);
            User{created_at:utc,variant:Variant::One, ..Default::default()}
        }
    }
    
     //#[sqlx(type_name = "variant")] или реализовать sqlx::Type::type_info
     #[derive(Debug,PartialEq,PartialOrd)]
     pub enum Variant{
         One,
         Two
     }

    impl sqlx::Type<sqlx::Postgres> for Variant {
        fn type_info() -> sqlx::postgres::PgTypeInfo {
            // если поле существует в базе данных, тип выведется из указанного имени
            // sqlx::postgres::PgTypeInfo::with_name("variant")
            // иначе вернуть тип поля самостоятельно
            sqlx::postgres::PgTypeInfo::with_name("TEXT")//типы из sqlx::postgres::PgTypeInfo::TEXT
        }
        fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
            *ty == Self::type_info()
        }
    }
 
    // implementation Encode
    impl<'q, DB: sqlx::Database> Encode<'q, DB> for Variant
    where
        &'q str: Encode<'q, DB>,
    {
        fn encode_by_ref(
            &self,
            buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
        ) -> sqlx::encode::IsNull {
            let val = match self {
                Variant::One => "One",
                Variant::Two => "Two",
            };
            <&str as Encode<'q, DB>>::encode(val, buf)
        }
        fn size_hint(&self) -> usize {
            let val = match self {
                Variant::One => "One",
                Variant::Two => "Two",
            };
            <&str as Encode<'q, DB>>::size_hint(&val)
        }
    }
    // implementation Decode
    impl<'r> Decode<'r, sqlx::postgres::Postgres> for Variant {
        fn decode(
            value: sqlx::postgres::PgValueRef<'r>,
        ) -> Result< Self, Box< dyn Error + 'static + Send + Sync,>, > {
            let value = <&'r str as Decode< 'r,sqlx::postgres::Postgres, >>::decode(value)?;
            match value {
                "One" | "one" => Result::Ok(Variant::One),
                "Two" | "two" => Result::Ok(Variant::Two),
                _ => Err({format!("invalid value {} for enum Variant",&value)}.into()),
            }
        }
    }

    // Для возможности преобразовать результат запроса при sqlx::query_as
    impl<'a, R: sqlx::Row> sqlx::FromRow<'a, R> for Variant
    where
        &'a str: sqlx::ColumnIndex<R>, 
        Variant: Decode<'a, R::Database>,
        Variant: Type<R::Database>, usize: sqlx::ColumnIndex<R>,  
    {
        fn from_row(row: &'a R) -> sqlx::Result<Self> {  
            let variant:Variant = row.try_get(/*0_usize*/"variant")?; // variant это если есть такое название поля в базе
            Result::Ok(variant)
        }
    }

     // Для типов не реализующих sqlx::decode::Decode следует реализовать самостоятельно
     // Для возможности преобразовать результат запроса при sqlx::query_as
     // Добавил features=["chrono"] для типа DateTime 
    impl<'a, R: sqlx::Row> sqlx::FromRow<'a, R> for User
    where
        &'a str: sqlx::ColumnIndex<R>,
        String: Decode<'a, R::Database>,
        String: Type<R::Database>,
        i32: Decode<'a, R::Database>,
        i32: Type<R::Database>,
        DateTime<Local>: Decode<'a, R::Database>,
        DateTime<Local>: Type<R::Database>,
        Variant: Decode<'a, R::Database>,
        Variant: Type<R::Database>, usize: sqlx::ColumnIndex<R>,
       // Id: Decode<'a, R::Database>,
       // Id: Type<R::Database>,
        Id: Type<<R as Row>::Database>
    {
        fn from_row(row: &'a R) -> sqlx::Result<Self> {
            let name: String = row.try_get("name")?;
            let id: Id = row.try_get("id")?;
            let created_at: DateTime<Local> = row.try_get("created_at")?;
            let variant:Variant = row.try_get(/*3_usize*/"variant")?;  // variant это если есть такое название поля в базе
            Result::Ok(User { name, id, created_at,variant })
        }
    }
}


//-----------------------------------------------------------------------------------------------------------------
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Bad config: {0}")]
    BadConfig(String),
    #[error("Connecting to database: {0}")]
    ConnectingToDatabase(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Migrating database: {0}")]
    DatabaseMigration(String),
}

impl std::convert::From<sqlx::Error> for self::Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => self::Error::NotFound("row not found".into()),
            _ => self::Error::Internal(err.to_string()),
        }
    }
}

impl std::convert::From<sqlx::migrate::MigrateError> for self::Error {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        self::Error::DatabaseMigration(err.to_string())
    }
}