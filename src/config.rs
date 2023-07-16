use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{env, fs, path};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub bot_token: String,
    #[serde(default = "redis_addr_default")]
    pub redis_addr: String,
    #[serde(default = "log_level_default")]
    pub log_level: String,
    #[serde(default = "health_check_port_default")]
    pub health_check_port: u16,

    pub deepl: DeepLConfig,
    pub osu: OsuConfig,

    pub bili_live_room_event: HashMap<String, Vec<u64>>,
    pub osu_user_activity_event: HashMap<String, Vec<String>>,
}

impl Config {
    fn get_config_dir() -> anyhow::Result<path::PathBuf> {
        let config_dir = if let Ok(xdg_path) = env::var("XDG_CONFIG_HOME") {
            path::PathBuf::from(&xdg_path)
        } else {
            path::Path::new(&env::var("HOME").unwrap()).join(".config")
        };

        let dir = config_dir.join("tg_maid");

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }

    pub fn from_path() -> anyhow::Result<Self> {
        let file_path = if let Ok(cfg_path) = env::var("TG_MAID_CFG_PATH") {
            path::PathBuf::from(cfg_path)
        } else {
            Self::get_config_dir()
                .with_context(|| "fail to open config directory")?
                .join("config.toml")
        };

        if !file_path.exists() {
            anyhow::bail!("Config file not found in {file_path:?}");
        }
        let content = fs::read_to_string(file_path).with_context(|| "fail to read config file")?;

        toml::from_str(&content).with_context(|| "fail to parse config from toml")
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeepLConfig {
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OsuConfig {
    pub client_id: u64,
    pub client_secret: String,
}

fn redis_addr_default() -> String {
    "redis://localhost:6379".to_string()
}

fn health_check_port_default() -> u16 {
    11451
}

fn log_level_default() -> String {
    "INFO".to_string()
}

#[test]
fn validate_file_correctness() {
    std::env::set_var("XDG_CONFIG_HOME", env::temp_dir().join("tg-maid-test-dir"));
    let config = r#"
        bot_token = "abcde"
        redis_addr = "redis://localhost"
        log_level = "INFO"
        health_check_port = 11451

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

    let config = Config::from_path().unwrap();
    assert_eq!(config.bot_token, "abcde");

    fs::remove_dir(env::temp_dir().join("tg-maid-test-dir")).unwrap();
}
