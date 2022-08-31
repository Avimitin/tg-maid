#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use teloxide::prelude::*;

const ENDPOINT: &str = "https://api.live.bilibili.com/room/v1/Room/get_status_info_by_uids";

struct Runtime {
    c: reqwest::Client,
    b: AutoSend<Bot>,
    cfg: Config,
}

pub struct Config {
    pub watch: Vec<u32>,
    pub notify: Vec<ChatId>,
}

type BoxStr = Box<str>;

#[derive(Deserialize, Debug)]
struct Response {
    code: u8,
    msg: BoxStr,
    message: BoxStr,
    data: HashMap<BoxStr, RoomInfo>,
}

#[derive(Deserialize, Debug)]
struct RoomInfo {
    title: BoxStr,
    cover_from_user: BoxStr,
    keyframe: BoxStr,
    live_status: u8,
    #[serde(rename = "uname")]
    username: BoxStr,
    area_v2_name: BoxStr,
}

impl RoomInfo {
    fn to_captions(&self) -> String {
        format!(
            "{} 开播了！\n直播标题: {}\n分区: #{}\n",
            self.username, self.title, self.area_v2_name
        )
    }
}

async fn get_room_info(
    client: &reqwest::Client,
    uids: &[u32],
) -> anyhow::Result<HashMap<BoxStr, RoomInfo>> {
    let payload = HashMap::from([("uids", uids)]);
    let res = client
        .post(ENDPOINT)
        .json(&payload)
        .send()
        .await?
        .json::<Response>()
        .await?;

    Ok(res.data)
}

async fn watch_and_response(rt: Arc<Runtime>) {
    let res = get_room_info(&rt.c, &rt.cfg.watch).await;
    match res {
        Ok(rooms) => {
            for (_, room_info) in rooms {
                for chat in &rt.cfg.notify {
                    let cover = reqwest::Url::parse(&room_info.cover_from_user);
                    if cover.is_err() {
                        break;
                    }
                    let cover = cover.unwrap();

                    if let Err(err) =
                        rt.b.send_photo(*chat, teloxide::types::InputFile::url(cover))
                            .caption(room_info.to_captions())
                            .await
                    {
                        tracing::error!("fail to notify bili info: {err}")
                    }
                }
            }
        }
        Err(err) => {
            tracing::error!("fail to get information from bilibili: {err}")
        }
    }
}

pub fn spawn_watcher(cfg: Config, b: AutoSend<Bot>) {
    let (tx, rx) = tokio::sync::watch::channel(1_u8);
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(60));
    let rt = Arc::new(Runtime {
        c: reqwest::Client::new(),
        b,
        cfg,
    });

    tokio::spawn(async move {
        loop {
            let mut rx = rx.clone();
            let rt = Arc::clone(&rt);

            tokio::select! {
                _ = rx.changed() => {
                    break;
                }
                _ = heartbeat.tick() => {
                    watch_and_response(rt).await;
                }
            }
        }
    });

    tokio::spawn(super::notify_on_ctrl_c(tx));
}

#[tokio::test]
async fn test_response() {
    let mut map = HashMap::new();
    map.insert("uids", vec![672328094]);

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.live.bilibili.com/room/v1/Room/get_status_info_by_uids")
        .json(&map)
        .send()
        .await
        .unwrap()
        .json::<Response>()
        .await
        .unwrap();

    println!("{:?}", res);
}
