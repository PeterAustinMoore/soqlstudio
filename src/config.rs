use serde_derive::{Deserialize, Serialize};

const CONFIG_JSON_FILE_PATH: &str = "config.json";

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub domain: String,
    pub dataset: String,
    pub query: String
}

pub fn get_config() -> Config {
    let raw_config = std::fs::read_to_string(CONFIG_JSON_FILE_PATH).unwrap_or(String::from(r#"
    {"username":"", "password":"", "domain":"", "dataset":"", "query":"","theme":"light"}
    "#));
    serde_json::from_str::<Config>(&raw_config).unwrap_or(Config {
        username: "".to_owned(),
        password: "".to_owned(),
        domain: "".to_owned(),
        dataset: "".to_owned(),
        query: "".to_owned(),
    })
}


pub fn set_config(new_config: Config) {
    std::fs::write(
        CONFIG_JSON_FILE_PATH,
        serde_json::to_string_pretty(&new_config).unwrap(),
    ).unwrap();
}
