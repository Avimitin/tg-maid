use crate::modules::runtime::Runtime;
use anyhow::Result;
use redis::aio::ConnectionManager as redis_cm;
use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::payloads::SendPhotoSetters;
use teloxide::prelude::*;

use crate::modules::scraper;
use crate::modules::types::CurrenciesStorage;

// Thanks to Asuna again! (GitHub @SpriteOvO)
macro_rules! generate_commands {
    ($($($cmd:ident)::+$(($($param:ident),+ $(,)?))* -> $endpoint:expr); +) => {
        teloxide::filter_command::<Command, _>()
        $(
            .branch(dptree::case![$($cmd)::+$(($($param,)*))*].endpoint($endpoint))
        )+
    }
}

#[derive(Clone)]
pub enum DialogueStatus {
    None,
    CmdCollectRunning,
}

impl std::default::Default for DialogueStatus {
    fn default() -> Self {
        Self::None
    }
}

type Dialogue = dialogue::Dialogue<DialogueStatus, dialogue::InMemStorage<DialogueStatus>>;
type RedisRT = Runtime<redis_cm, redis_cm>;

pub fn handler_schema() -> UpdateHandler<anyhow::Error> {
    use crate::modules::commands::Command;

    let stateless_cmd_handler = generate_commands! {
        Command::Exchange(amount, from, to) -> exchange_handler;
        Command::Help                       -> help_handler;
        Command::Weather                    -> weather_handler;
        Command::Ghs                        -> ghs_handler;
        Command::Mjx                        -> mjx_handler;
        Command::Collect                    -> collect_handler;
        Command::CookPiggy                  -> cook_piggy_handler
    };

    let stateful_cmd_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::CollectDone].endpoint(exit_collect_handler));

    // * is_message -> no-status -> stateless_cmd_handler
    //              -> collect-status -> is-command -> stateful_cmd_handler
    //                                -> collect_handler
    let msg_handler = Update::filter_message()
        .branch(dptree::case![DialogueStatus::None].branch(stateless_cmd_handler))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].branch(stateful_cmd_handler))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].endpoint(collect_message));

    let root = dptree::entry().branch(msg_handler);

    dialogue::enter::<Update, dialogue::InMemStorage<DialogueStatus>, DialogueStatus, _>()
        .branch(root)
}

async fn mjx_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::UploadPhoto)
        .await?;

    let resp = rt.req.get_mjx().await;

    match resp {
        Ok(s) => {
            bot.send_photo(msg.chat.id, teloxide::types::InputFile::url(s))
                .await?
        }
        Err(e) => bot.send_message(msg.chat.id, e.to_string()).await?,
    };

    Ok(())
}

async fn collect_message(msg: Message, rt: RedisRT) -> Result<()> {
    let mut collector = rt.collector.lock().await;
    let who_want_these = msg
        .from()
        .expect("Unexpectedly add non-user into dialogue")
        .id
        .0;

    let msg_from = msg
        .forward_from_user()
        .ok_or_else(|| anyhow::anyhow!("no user given"))?
        .first_name
        .to_string();
    let msg_text = msg.text().unwrap_or("Null").to_string();

    use crate::modules::collect::{Collector, MsgForm};
    collector
        .push(who_want_these, MsgForm::new(msg_from, msg_text))
        .await?;
    Ok(())
}

async fn cook_piggy_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let page = rt.req.get_piggy_recipe().await;
    if let Err(e) = page {
        bot.send_message(msg.chat.id, format!("今天没法吃 piggy 了呜呜呜: {e}"))
            .await?;

        return Ok(());
    }

    let page = page.unwrap();

    // Deserialize HTML page is a heavy task, however we don't have async way to do
    // it. So what I can do is just not let the job block current thread.
    let task = move || -> String {
        let res = scraper::collect_recipe(&page);
        match res {
            Ok(v) => {
                use rand::Rng;
                let choice: usize = rand::thread_rng().gen_range(0..v.len());
                format!("今天我们这样吃 piggy: {}", v[choice])
            }
            Err(e) => format!("今天没法吃 piggy 了呜呜呜: {e}"),
        }
    };

    let text = tokio::task::block_in_place(task);

    bot.send_message(msg.chat.id, text).await?;

    Ok(())
}

async fn ghs_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::UploadPhoto)
        .await?;
    let resp = rt.req.konachan_explicit_nsfw_image().await;

    match resp {
        Ok((image_link, image_info)) => {
            bot.send_photo(msg.chat.id, teloxide::types::InputFile::url(image_link))
                .parse_mode(teloxide::types::ParseMode::Html)
                .caption(image_info)
                .await?
        }
        Err(e) => bot.send_message(msg.chat.id, e.to_string()).await?,
    };

    Ok(())
}

async fn collect_handler(msg: Message, bot: AutoSend<Bot>, dialogue: Dialogue) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "你可以开始转发信息了，使用命令 /collect_done 来结束命令收集",
    )
    .await?;
    dialogue.update(DialogueStatus::CmdCollectRunning).await?;
    Ok(())
}

async fn exit_collect_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    dialogue: Dialogue,
    rt: RedisRT,
) -> Result<()> {
    let msg_id = bot
        .send_message(msg.chat.id, "收集完毕，正在处理信息...")
        .await?
        .id;
    dialogue.exit().await?;

    let mut collector = rt.collector.lock().await;
    use super::collect::Collector;

    // FIXME: Can I guarantee that command must came from a user?
    let result = collector
        .finish(msg.from().expect("Message came from non-user").id.0)
        .await;
    match result {
        Some(s) => bot.edit_message_text(msg.chat.id, msg_id, s).await?,
        None => {
            bot.edit_message_text(msg.chat.id, msg_id, "你还没有收集过消息")
                .await?
        }
    };

    Ok(())
}

async fn calculate_exchange(rt: RedisRT, amount: f64, from: String, to: String) -> Result<String> {
    let mut cache = rt.currency_cache.lock().await;
    if !cache.verify_date().await {
        let code = rt.req.get_currency_codes().await?;
        cache.update_currency_codes(code).await;
    }

    let from_fullname = cache.get_fullname(&from).await;
    if from_fullname.is_none() {
        anyhow::bail!("invalid currency: {from}")
    }

    let to_fullname = cache.get_fullname(&to).await;
    if to_fullname.is_none() {
        anyhow::bail!("invalid currency: {to}")
    }

    // Early drop the MutexGuard to avoid the below request block the redis locker
    drop(cache);

    let rate_info = rt.req.get_currency_rate(&from, &to).await?;

    Ok(format!(
        r#"
<b>{}</b> --> <b>{}</b>

<b>{:.3}</b> {} = <b>{:.3}</b> {}

Date: {}
                           "#,
        from_fullname.unwrap(),
        to_fullname.unwrap(),
        amount,
        from.to_uppercase(),
        rate_info.rate * amount,
        to.to_uppercase(),
        rate_info.date
    ))
}

async fn exchange_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    rt: RedisRT,
    payload: (f64, String, String),
) -> Result<()> {
    let chat_id = msg.chat.id;

    let callback = bot.send_message(chat_id, "Fetching API...").await?;

    match calculate_exchange(rt, payload.0, payload.1, payload.2).await {
        Ok(reply) => {
            bot.edit_message_text(chat_id, callback.id, reply)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?
        }
        Err(e) => {
            bot.edit_message_text(
                chat_id,
                callback.id,
                format!("fail to calculate rate: \n{e}"),
            )
            .await?
        }
    };

    Ok(())
}

async fn help_handler(msg: Message, bot: AutoSend<Bot>) -> Result<()> {
    use crate::modules::commands::Command;
    use teloxide::utils::command::BotCommands;
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn get_weather(msg: Message, rt: RedisRT) -> Result<String> {
    let text = msg.text().unwrap();
    let parts = text.split(' ').collect::<Vec<&str>>();
    if parts.len() < 2 {
        anyhow::bail!("No enough argument. Usage example: /weather 上海")
    }

    let (text, pic) = rt.req.wttr_in_weather(parts[1]).await?;
    Ok(format!("<a href=\"{pic}\">{text}</a>"))
}

async fn weather_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let chat_id = msg.chat.id;
    bot.send_chat_action(chat_id, teloxide::types::ChatAction::Typing)
        .await?;
    let response = get_weather(msg, rt).await;

    match response {
        Ok(text) => {
            bot.send_message(chat_id, text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?
        }
        Err(e) => {
            bot.send_message(chat_id, format!("fail to get weather:\n{e}"))
                .await?
        }
    };

    Ok(())
}
