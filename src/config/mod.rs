
use std::env;
use std::sync::OnceLock;
use errors::Error;

mod errors {
    #[derive(Debug)]
    pub enum Error {
        ParamNotInEnv(&'static str),
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct Config {
    pub GOOGLE_PLACES_API: String,
    pub WEB_SERVER_URL: String,
}

pub fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();

    CONFIG.get_or_init(|| {
        Config::load_from_env().unwrap_or_else(|e| {
            panic!("PANIC -- loading config -- cause: {e:?}")
        })
    })
}

impl Config {
    pub fn load_from_env() -> Result<Config, Error> {
        Ok(Config {
            GOOGLE_PLACES_API: get_env_as_string("GOOGLE_PLACES_API")?,
            WEB_SERVER_URL: get_env_as_string("WEB_SERVER_URL")?
        })
    }
}

fn get_env_as_string(param: &'static str) -> Result<String, Error>{
    Ok(env::var(param).map_err(|_| Error::ParamNotInEnv(param))?)
}