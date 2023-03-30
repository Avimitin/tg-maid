use crate::app::AppData;
use redis::Commands;
use teloxide::prelude::Message;

use super::Sendable;

pub async fn push_msg(data: AppData, msg: Message) -> anyhow::Result<u32> {
    let requester = msg.from().unwrap().id.0;

    let sender = {
        if let Some(original_sender) = msg.forward_from_user() {
            original_sender.first_name.as_str()
        } else if let Some(original_sender_name) = msg.forward_from_sender_name() {
            original_sender_name
        } else {
            "Anoynomous"
        }
    };

    let text = {
        if let Some(text) = msg.text() {
            text
        } else if msg.video().is_some() {
            "[video]"
        } else if msg.audio().is_some() {
            "[audio]"
        } else if msg.sticker().is_some() {
            "[sticker]"
        } else if msg.photo().is_some() {
            "[photo]"
        } else {
            "Unsupported message type"
        }
    };

    // [2001-02-03] at 04:05:06
    let val = if let Some(date) = msg.forward_date() {
        format!("{}, <b>{sender}</b>:\n{text}", date.format("[%F] at %T"))
    } else {
        format!("<b>{sender}</b>:\n{text}")
    };

    let key = format!("TG_COMMAND:COLLECT:{requester}");
    let array_size = data.cacher.get_conn().rpush(key, val)?;
    Ok(array_size)
}

pub async fn finish(data: AppData, msg: &Message) -> anyhow::Result<Sendable> {
    let uid = msg.from().unwrap().id.0;
    let key = format!("TG_COMMAND:COLLECT:{uid}");
    let mut redis = data.cacher.get_conn();
    let all: Vec<String> = redis.lrange(&key, 0, -1)?;
    redis.del(&key)?;

    Ok(Sendable::Text(all.join("\n")))
}
