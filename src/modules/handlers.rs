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

// Bot action wrapper
macro_rules! send {
    ($msg:ident, $bot:ident, $text:literal) => {
        $bot.send_message($msg.chat.id, $text).await?
    };

    ($msg:ident, $bot:ident, $text:literal, html) => {
        $bot.send_message($msg.chat.id, $text)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?
    };

    ($msg:ident, $bot:ident, $text:expr) => {
        $bot.send_message($msg.chat.id, $text).await?
    };

    (@$action:ident; $msg:ident, $bot:ident) => {
        $bot.send_chat_action($msg.chat.id, teloxide::types::ChatAction::$action).await?
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
type RedisRT = Runtime<redis_cm, redis_cm, redis_cm>;

pub fn handler_schema() -> UpdateHandler<anyhow::Error> {
    use crate::modules::commands::Command;

    let stateless_cmd_handler = generate_commands! {
        Command::Exchange(amount, from, to) -> exchange_handler;
        Command::Help                       -> help_handler;
        Command::Weather                    -> weather_handler;
        Command::Ghs                        -> ghs_handler;
        Command::Mjx                        -> mjx_handler;
        Command::Collect                    -> collect_handler;
        Command::CookPiggy                  -> cook_piggy_handler;
        Command::HitKsyx                    -> ksyx_handler;
        Command::Eh                         -> eh_handler;
        Command::EhSeed                     -> eh_seed_handler;
        Command::Pacman                     -> pacman_handler
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

async fn pacman_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let mut text = msg.text().unwrap().split(' ');
    // shift one
    text.next();

    let operation = text.next();
    if operation.is_none() {
        send!(msg, bot, "No operation was given, abort!");
        return Ok(());
    }
    let operation = operation.unwrap();

    let pkg = text.next();
    if pkg.is_none() {
        send!(msg, bot, "No package name! Abort");
        return Ok(());
    }

    match operation {
        "-S" => {
            let resp = rt.req.exact_match(pkg.unwrap()).await;
            match resp {
                Ok(s) => send!(msg, bot, format!("{s}")),
                Err(e) => send!(msg, bot, format!("{e}")),
            }
        }
        _ => {
            send!(
                msg,
                bot,
                format!("Unsupported operation `{operation}`! Abort")
            )
        }
    };

    send!(@Typing; msg, bot);
    Ok(())
}

async fn ksyx_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let mut conn = rt.ksyx_hit_counter.lock().await;
    use crate::modules::ksyx::KsyxCounter;
    let old_v = conn.add().await;
    if let Err(ref e) = old_v {
        bot.send_message(msg.chat.id, format!("fail to interact with ksyx: {e}"))
            .await?;
        return Ok(());
    }

    let action = &[
        "爱抚", "中出", "暴打", "后入", "膜", "贴贴", "狂踹", "寸止", "绳缚",
    ];
    use rand::Rng;
    let choice = rand::thread_rng().gen_range(0..action.len());
    bot.send_message(
        msg.chat.id,
        format!(
            "{} {}了 ksyx，ksyx 已经被动手动脚了 {} 次",
            msg.from().unwrap().first_name,
            action[choice],
            old_v.unwrap(),
        ),
    )
    .await?;

    Ok(())
}

async fn parse_eh_gidlist(msg: &Message, bot: &AutoSend<Bot>) -> Result<Vec<[String; 2]>> {
    bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::UploadPhoto)
        .await?;

    let text = msg.text().unwrap();
    let args = text.split_once(' ');

    let parse_uid = |args: &str| -> Vec<[String; 2]> {
        let rules = regex::Regex::new(r#"e.hentai\.org/g/(\d+)/(\w+)"#).unwrap();
        let mut ret = Vec::new();
        for cap in rules.captures_iter(args) {
            ret.push([cap[1].to_string(), cap[2].to_string()])
        }
        ret
    };

    let gid_list;
    if let Some(args) = args {
        let args = args.1;
        gid_list = parse_uid(args);
    } else {
        if msg.reply_to_message().is_none() {
            bot.send_message(msg.chat.id,
                "You need to attach a ehentai link, or reply to a message that contains the ehentai link.")
                .await?;
            anyhow::bail!("no link found")
        }

        let text = msg.reply_to_message().unwrap().text();
        if text.is_none() {
            bot.send_message(msg.chat.id, "You need to reply to a text message!")
                .await?;
            anyhow::bail!("no link found")
        }
        gid_list = parse_uid(text.unwrap());
    }

    if gid_list.is_empty() {
        bot.send_message(msg.chat.id, "No valid Ehentai or Exhentai link were found.")
            .await?;
    }

    Ok(gid_list)
}

async fn eh_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let gid_list = parse_eh_gidlist(&msg, &bot).await?;
    let response = rt.req.query_eh_api(&gid_list).await;
    match response {
        Ok(resp) => {
            if resp.gmetadata.is_empty() {
                bot.send_message(msg.chat.id, "invalid eh link").await?;
                return Ok(());
            }
            // TODO: support render multiple comic someday
            let metadata = &resp.gmetadata[0];
            bot.send_photo(
                msg.chat.id,
                teloxide::types::InputFile::url(metadata.thumb.clone()),
            )
            .caption(format!("{metadata}"))
            .await?;
        }
        Err(error) => {
            bot.send_message(msg.chat.id, format!("Query fail: {error}"))
                .await?;
        }
    };

    Ok(())
}

async fn eh_seed_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let gid_list = parse_eh_gidlist(&msg, &bot).await?;

    let response = rt.req.query_eh_api(&gid_list).await;
    match response {
        Ok(resp) => {
            if resp.gmetadata.is_empty() {
                bot.send_message(msg.chat.id, "invalid eh link").await?;
                return Ok(());
            }

            // TODO: support render multiple comic someday
            let metadata = &resp.gmetadata[0];
            // take 5 to avoid long message
            bot.send_message(msg.chat.id, metadata.to_telegram_html(5))
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Err(error) => {
            bot.send_message(msg.chat.id, format!("Query fail: {error}"))
                .await?;
        }
    }

    Ok(())
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

    let (amount, from, to) = payload;
    match calculate_exchange(rt, amount, from.to_lowercase(), to.to_lowercase()).await {
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
