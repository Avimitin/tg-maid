use crate::{app::AppData, config::Config, event::EventWatcher};
use redis::Commands;
use serde::Deserialize;
use std::collections::HashMap;
use teloxide::{payloads::SendPhotoSetters, prelude::Requester, types as tg_type};

pub struct BiliApi;
impl BiliApi {
    const BATCH_ROOM_INFO: &'static str =
        "https://api.live.bilibili.com/room/v1/Room/get_status_info_by_uids";
}

#[derive(Deserialize, Debug)]
struct Response {
    code: u8,
    message: String,
    data: HashMap<String, RoomInfo>,
}

pub fn spawn_bilibili_live_room_listener(bot: teloxide::Bot, data: AppData, config: &Config) {
    EventWatcher::builder()
        .name("BilibiliLiveRoomWatcher")
        .bot(bot)
        .data(data)
        .heartbeat_interval(600) // 10 mins
        .build()
        .setup_subscribe_registry(config.bili_live_room_event.iter())
        .start_with_task(watch_and_response);
}

#[derive(Deserialize, Debug)]
pub struct RoomInfo {
    title: String,
    cover_from_user: String,
    live_status: u8,
    #[serde(rename = "uname")]
    username: String,
    area_v2_name: String,
    room_id: u32,
    uid: u64,
    online: u64,
    keyframe: String,
}

impl RoomInfo {
    fn to_captions(&self, status: u8) -> Option<String> {
        match status {
            0 => Some(format!(
                "{} 下播了！",
                self.username,
            )),
            1 => Some(format!(
                "<a href=\"{}\">{}</a> 开播了！已有 {} 人正在观看\n直播: <a href=\"{}\">{}</a>\n分区: #{}\n",
                format_args!("https://space.bilibili.com/{}/", self.uid),
                self.username,
                self.online,
                format_args!("https://live.bilibili.com/{}/", self.room_id),
                self.title,
                self.area_v2_name
            )),
            _ => None,
        }
    }
}

pub async fn batch_get_room_info(
    data: &AppData,
    user_ids: impl Iterator<Item = &u64>,
) -> anyhow::Result<HashMap<String, RoomInfo>> {
    let payload = HashMap::from([("uids", user_ids.collect::<Vec<_>>())]);
    let info = data
        .requester
        .post_json_to_t::<Response>(&payload, BiliApi::BATCH_ROOM_INFO)
        .await?;

    if info.code != 0 {
        anyhow::bail!("{}", info.message);
    }

    Ok(info.data)
}

pub fn cache_bili_live_room_status(data: &AppData, info: &RoomInfo) -> anyhow::Result<u8> {
    let key = format!("BILI_LIVE_ROOM_STATUS:{}", info.room_id);
    let mut conn = data.cacher.get_conn();
    let prev_status: Option<u8> = conn.get(&key)?;

    let () = conn.set(&key, info.live_status)?;

    // 255 indicate that the status is not exist before
    Ok(prev_status.unwrap_or(255))
}

async fn watch_and_response(ctx: EventWatcher<()>) -> anyhow::Result<()> {
    let subscribed_rooms = ctx.event_pool()?;
    let response = batch_get_room_info(&ctx.data, subscribed_rooms.iter()).await?;

    for (_, room_info) in response {
        let prev_status = cache_bili_live_room_status(&ctx.data, &room_info);
        if let Err(err) = prev_status {
            tracing::error!("[BiliLiveRoom] fail to update cache: {err}");
            continue;
        }

        let status_unchanged = prev_status.unwrap() == room_info.live_status;
        if status_unchanged {
            continue;
        }

        let subscribers = ctx.get_subscribers(&room_info.uid)?;
        for chat_id in subscribers {
            if let Err(err) = notify_live_room_changes(&ctx.bot, chat_id, &room_info).await {
                tracing::error!("[BiliLiveRoom] fail to notify changes: {err}")
            }
        }
    }

    Ok(())
}

async fn notify_live_room_changes(
    bot: &teloxide::Bot,
    chat_id: i64,
    room_info: &RoomInfo,
) -> anyhow::Result<()> {
    let cover = if room_info.live_status == 0 {
        reqwest::Url::parse(&room_info.keyframe)?
    } else {
        reqwest::Url::parse(&room_info.cover_from_user)?
    };

    let caption = room_info.to_captions(room_info.live_status);
    if caption.is_none() {
        return Ok(());
    }
    let caption = caption.unwrap();
    bot.send_photo(tg_type::ChatId(chat_id), tg_type::InputFile::url(cover))
        .caption(caption)
        .parse_mode(tg_type::ParseMode::Html)
        .await?;
    Ok(())
}
