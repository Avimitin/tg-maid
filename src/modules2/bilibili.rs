use crate::app::AppData;
use redis::Commands;
use serde::Deserialize;
use std::collections::HashMap;

pub struct BiliApi;
impl BiliApi {
    const BATCH_ROOM_INFO: &str =
        "https://api.live.bilibili.com/room/v1/Room/get_status_info_by_uids";
}

#[derive(Deserialize, Debug)]
struct Response {
    code: u8,
    message: String,
    data: HashMap<String, RoomInfo>,
}

#[derive(Deserialize, Debug)]
pub struct RoomInfo {
    title: String,
    cover_from_user: String,
    keyframe: String,
    live_status: u8,
    #[serde(rename = "uname")]
    username: String,
    area_v2_name: String,
    room_id: u32,
    uid: u32,
}

pub async fn batch_get_room_info(
    data: AppData,
    user_ids: impl Iterator<Item = u32>,
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

pub async fn cache_bili_live_room_status(data: &AppData, info: &RoomInfo) -> anyhow::Result<u8> {
    let key = format!("BILI_LIVE_ROOM_STATUS:{}", info.room_id);
    let mut conn = data.cacher.get_conn();
    let prev_status: Option<u8> = conn.get(&key)?;

    conn.set(&key, info.live_status)?;

    // 255 indicate that the status is not exist before
    Ok(prev_status.unwrap_or(255))
}
