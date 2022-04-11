use ini::Ini;
use lazy_static::lazy_static;

pub static WEATHER_SERVER_ADDR_KEY: &str = "WEATHER_SERVER_ADDRESS";

lazy_static! {
    pub static ref CONFIG: Ini = Ini::load_from_file(".weather_server_config").unwrap();
}
