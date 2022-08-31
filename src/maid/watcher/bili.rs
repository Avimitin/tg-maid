#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;

const ENDPOINT: &str = "http://api.live.bilibili.com/room/v1/Room/get_status_info_by_uids";

struct Runtime {
    c: reqwest::Client,
}

struct Config {
    watch: Vec<u32>,
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

async fn get_room_info(
    rt: Arc<Runtime>,
    uids: &[u32],
) -> anyhow::Result<HashMap<BoxStr, RoomInfo>> {
    let payload = HashMap::from([("uids", uids)]);
    let res =
        rt.c.post(ENDPOINT)
            .json(&payload)
            .send()
            .await?
            .json::<Response>()
            .await?;

    Ok(res.data)
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
