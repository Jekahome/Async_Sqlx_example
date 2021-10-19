
pub mod settings{
    use config::{Config};
    use std::sync::RwLock;
    use lazy_static::lazy_static;
    use sqlx::postgres::PgConnectOptions;

    lazy_static! {
        static ref SETTINGS: RwLock<Config> = {
        let mut settings: Config = Config::default();
            settings
            .merge(config::File::with_name("settings/settings.toml")).unwrap()
            .merge(config::Environment::with_prefix("APP")).unwrap();
            RwLock::new(settings)
        };
    }

   pub fn config() -> Result<String, Box<dyn std::error::Error>>{
        let config = format!("postgres://{user}:{password}@{host}:{port}/{dbname}",
        host=SETTINGS.read()?.get::<String>("host")?,
        user=SETTINGS.read()?.get::<String>("user")?,
        port=SETTINGS.read()?.get::<i32>("port")?,
        password=SETTINGS.read()?.get::<String>("password")?,
        dbname=SETTINGS.read()?.get::<String>("dbname")?);
        Ok(config)
   } 

   pub fn config_2() -> Result<PgConnectOptions, Box<dyn std::error::Error>>{    
        let pg_conn_option = PgConnectOptions::new()
        .host(&SETTINGS.read()?.get::<String>("host")?)
        .port(SETTINGS.read()?.get::<u16>("port")?)
        .username(&SETTINGS.read()?.get::<String>("user")?)
        .password(&SETTINGS.read()?.get::<String>("password")?)
        .ssl_mode(sqlx::postgres::PgSslMode::Disable);

    Ok(pg_conn_option)
   } 
}