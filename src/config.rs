use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{env, fs, path};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub bot_token: String,
    pub redis_addr: String,
    pub log_level: String,

    pub deepl: DeepLConfig,
    pub osu: OsuConfig,

    pub bili_live_room_event: HashMap<String, Vec<u32>>,
    pub osu_user_activity_event: HashMap<String, Vec<String>>,
}

impl Config {
    fn get_config_dir() -> path::PathBuf {
        let config_dir = if let Ok(xdg_path) = env::var("XDG_CONFIG_HOME") {
            path::PathBuf::from(&xdg_path)
        } else {
            path::Path::new(&env::var("HOME").unwrap()).join(".config")
        };

        let dir = config_dir.join("tg_maid");

        if !dir.exists() {
            fs::create_dir_all(&dir).expect("fail to create config dir");
        }

        dir
    }

    pub fn new() -> Self {
        let file_path = Self::get_config_dir().join("config.toml");
        if !file_path.exists() {
            panic!("No config file found");
        }
        let content = fs::read_to_string(file_path).expect("fail to read config");
        toml::from_str(&content).expect("invalid toml content")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeepLConfig {
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OsuConfig {
    pub client_id: u32,
    pub client_secret: String,
}

#[test]
fn validate_file_correctness() {
    std::env::set_var("XDG_CONFIG_HOME", env::temp_dir().join("tg-maid-test-dir"));
    let config = r#"
        bot_token = "abcde"
        redis_addr = "redis://localhost"
        log_level = "INFO"

        [deepl]
        api_key = "abcde"

        [osu]
        client_id = 12345
        client_secret = "abcde"

        [bili_live_room_event]
        "-10012345" = [ 1000, 2000, 3000 ]
        "-10054321" = [ 1000, 2000, 3000 ]

        [osu_user_activity_event]
        "-10012345" = [ "Cookiezi", "Rafis" ]
        "-10054321" = [ "WhiteCat", "Mrekk" ]
    "#;
    let path = env::temp_dir().join("tg-maid-test-dir").join("tg_maid");
    fs::create_dir_all(env::temp_dir().join("tg-maid-test-dir").join("tg_maid")).unwrap();
    fs::write(path.join("config.toml"), config).unwrap();

    let config = Config::new();
    assert_eq!(config.bot_token, "abcde");

    fs::remove_dir(env::temp_dir().join("tg-maid-test-dir")).unwrap();
}
