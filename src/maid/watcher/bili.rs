#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use teloxide::prelude::*;
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://api.live.bilibili.com/room/v1/Room/get_status_info_by_uids";

struct Runtime<T: BiliRoomQueryCache> {
    c: reqwest::Client,
    b: Bot,
    cfg: Config,
    cache: Mutex<T>,
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
    room_id: u32,
    uid: u32,
}

impl RoomInfo {
    fn to_captions(&self, status: u8) -> Option<String> {
        match status {
            0 => Some(format!("{} 下播了！", self.username)),
            1 => Some(format!(
                "<a href=\"{}\">{}</a> 开播了！\n直播: <a href=\"{}\">{}</a>\n分区: #{}\n",
                format_args!("https://space.bilibili.com/{}/", self.uid),
                self.username,
                format_args!("https://live.bilibili.com/{}/", self.room_id),
                self.title,
                self.area_v2_name
            )),
            _ => None,
        }
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

#[async_trait::async_trait]
pub trait BiliRoomQueryCache: Send + Sync {
    async fn update_status(&mut self, room_id: u32, status: u8) -> anyhow::Result<u8>;
}

async fn watch_and_response<T: BiliRoomQueryCache>(rt: Arc<Runtime<T>>) {
    let res = get_room_info(&rt.c, &rt.cfg.watch).await;
    if let Err(err) = res {
        tracing::error!("fail to get information from bilibili: {err}");
        return;
    }
    let rooms = res.unwrap();
    let mut cache = rt.cache.lock().await;
    for (_, room_info) in rooms {
        let last_status = cache
            .update_status(room_info.room_id, room_info.live_status)
            .await;
        if let Err(err) = last_status {
            tracing::error!("fail to update bilibili room cache: {err}");
            continue;
        }

        let last_status = last_status.unwrap();
        if last_status == room_info.live_status {
            continue;
        }

        for chat in &rt.cfg.notify {
            let cover = reqwest::Url::parse(&room_info.cover_from_user);
            if cover.is_err() {
                tracing::error!("get invalid cover image URL: {cover:?}");
                break;
            }
            let cover = cover.unwrap();

            let caption = room_info.to_captions(room_info.live_status);
            if caption.is_none() {
                continue;
            }
            let caption = caption.unwrap();

            if let Err(err) =
                rt.b.send_photo(*chat, teloxide::types::InputFile::url(cover))
                    .caption(caption)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await
            {
                tracing::error!("fail to notify bili info: {err}")
            }
        }
    }
}

pub fn spawn_watcher<C: BiliRoomQueryCache + 'static>(cfg: Config, b: Bot, cache: C) {
    let (tx, rx) = tokio::sync::watch::channel(1_u8);
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(60));
    let rt = Arc::new(Runtime {
        c: reqwest::Client::new(),
        b,
        cfg,
        cache: Mutex::new(cache),
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
