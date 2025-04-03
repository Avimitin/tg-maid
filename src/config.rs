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

    pub bili_live_room_event: HashMap<String, Vec<u64>>,

    #[serde(default = "proxy_default")]
    pub proxy: ProxyConfig,
}

impl Config {
    fn get_config_dir() -> anyhow::Result<path::PathBuf> {
        let config_dir = if let Ok(xdg_path) = env::var("XDG_CONFIG_HOME") {
            path::PathBuf::from(&xdg_path)
        } else if let Ok(home_dir) = env::var("HOME") {
            // windows has not "HOME"
            path::PathBuf::from(&home_dir).join(".config")
        } else {
            env::current_dir()?
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

#[derive(Debug, Serialize)]
pub enum ProxyType {
    UseDefault(bool),
    ProxyUrl(String),
}

impl<'de> Deserialize<'de> for ProxyType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        use toml::Value;
        let s = Value::deserialize(deserializer)?;
        match s {
            Value::String(url) => Ok(ProxyType::ProxyUrl(url)),
            Value::Boolean(bool) => Ok(ProxyType::UseDefault(bool)),
            _ => Err(Error::custom("Invalid proxy type")),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProxyConfig {
    default: Option<String>,
    telegram: Option<ProxyType>,
    deepl: Option<ProxyType>,
    bilibili: Option<ProxyType>,
}

macro_rules! proxy_getter_generate {
    ($field:ident) => {
        impl ProxyConfig {
            pub fn $field(&self) -> Option<&str> {
                if self.$field.is_none() {
                    return None;
                }
                match self.$field.as_ref()? {
                    ProxyType::UseDefault(use_default) => {
                        if !use_default {
                            return None;
                        }
                        if let Some(default) = &self.default {
                            Some(default)
                        } else {
                            None
                        }
                    }
                    ProxyType::ProxyUrl(url) => Some(url),
                }
            }
        }
    };
}
proxy_getter_generate!(telegram);
proxy_getter_generate!(deepl);
proxy_getter_generate!(bilibili);

fn redis_addr_default() -> String {
    "redis://localhost:6379".to_string()
}

fn health_check_port_default() -> u16 {
    11451
}

fn log_level_default() -> String {
    "INFO".to_string()
}

fn proxy_default() -> ProxyConfig {
    ProxyConfig {
        default: None,
        telegram: None,
        deepl: None,
        bilibili: None,
    }
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

        [bili_live_room_event]
        "-10012345" = [ 1000, 2000, 3000 ]
        "-10054321" = [ 1000, 2000, 3000 ]
    "#;
    let path = env::temp_dir().join("tg-maid-test-dir").join("tg_maid");
    fs::create_dir_all(env::temp_dir().join("tg-maid-test-dir").join("tg_maid")).unwrap();
    fs::write(path.join("config.toml"), config).unwrap();

    let config = Config::from_path().unwrap();
    assert_eq!(config.bot_token, "abcde");

    fs::remove_dir(env::temp_dir().join("tg-maid-test-dir")).unwrap();
}
