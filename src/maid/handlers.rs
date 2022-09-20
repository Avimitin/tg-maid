use super::runtime::Runtime;
use crate::modules;
use anyhow::Result;
use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::payloads::SendPhotoSetters;
use teloxide::prelude::*;

use crate::maid::Fetcher;

/// Generate relation from command literal to their corresponding endpoint.
///
/// # Form
///
/// ```
/// generate_commands!{
///     Command::Help  -> help_handler
/// }
/// ```
///
/// # Credit
///
/// Thanks to Asuna again! (GitHub @SpriteOvO)
///
macro_rules! generate_commands {
    ($($($cmd:ident)::+$(($($param:ident),+ $(,)?))* -> $endpoint:expr); +) => {
        teloxide::filter_command::<Command, _>()
        $(
            .branch(dptree::case![$($cmd)::+$(($($param,)*))*].endpoint($endpoint))
        )+
    }
}

/// A AutoSend<Bot> method wrapper. It can help reduce the code base.
///
/// Rules:
///    * send(Message, AutoSend<Bot>, &'static str): Send text message
///    * send(Message, AutoSend<Bot>, &'static str, html): Send text message in Html format
///    * send(Message, AutoSend<Bot>, {{ expression }}): Send text message, text generate from
///    expression
///    * send(@<Action>, Message, AutoSend<Bot>): Send chat action
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

    ($msg:ident, $bot:ident, $text:expr, html) => {
        $bot.send_message($msg.chat.id, $text)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?
    };

    (@$action:ident; $msg:ident, $bot:ident) => {
        $bot.send_chat_action($msg.chat.id, teloxide::types::ChatAction::$action)
            .await?
    };
}

/// Represent the bot status for the current requesting user.
#[derive(Clone)]
pub enum DialogueStatus {
    /// Normal status
    None,
    /// All the message from current user should be collected
    CmdCollectRunning,
}

impl std::default::Default for DialogueStatus {
    fn default() -> Self {
        Self::None
    }
}

type Dialogue = dialogue::Dialogue<DialogueStatus, dialogue::InMemStorage<DialogueStatus>>;

/// Runtime built with redis connection and reqwest::Client
type RedisRT = Runtime<redis::aio::ConnectionManager, Fetcher>;

/// Build the bot update handler schema.
///
/// # Logic
///
// * <is message?> -> <no status?> -> [stateless_cmd_handler]
//                   -> <is command?> -> [stateful cmd handler]
//                      -> [collect handler]
//
pub fn handler_schema() -> UpdateHandler<anyhow::Error> {
    use super::Command;

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
        Command::Pacman                     -> pacman_handler;
        Command::Id                         -> id_handler;
        Command::Translate                  -> translate_handler;
        Command::Tr                         -> translate_handler
    };

    let stateful_cmd_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::CollectDone].endpoint(exit_collect_handler));

    let msg_handler = Update::filter_message()
        .branch(dptree::case![DialogueStatus::None].branch(stateless_cmd_handler))
        .branch(dptree::case![DialogueStatus::None].endpoint(message_filter))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].branch(stateful_cmd_handler))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].endpoint(collect_message));

    let root = dptree::entry().branch(msg_handler);

    dialogue::enter::<Update, dialogue::InMemStorage<DialogueStatus>, DialogueStatus, _>()
        .branch(root)
}

async fn message_filter(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let text = msg.text();
    if text.is_none() {
        // silently exit
        return Ok(());
    }
    let text = text.unwrap();

    if let Some(resp) = rt.patterns.try_match(text) {
        bot.send_message(msg.chat.id, resp)
            .reply_to_message_id(msg.id)
            .await?;
    }

    Ok(())
}

async fn id_handler(msg: Message, bot: AutoSend<Bot>) -> Result<()> {
    let user_id = if let Some(reply) = msg.reply_to_message() {
        reply.from().map_or(0, |user| user.id.0)
    } else {
        msg.from().map_or(0, |user| user.id.0)
    };
    let chat_id = msg.chat.id;

    send!(msg, bot, format!("user id: {user_id}\nchat id: {chat_id}"));

    Ok(())
}

async fn translate_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    if msg.reply_to_message().is_none() {
        send!(
            msg,
            bot,
            r#"Usage: Reply to a text message and input
    <code>/tr [source language(optional)] target-language</code>.
Example:
    /tr zh en"#,
            html
        );
        return Ok(());
    }

    let text = msg.reply_to_message().unwrap().text();
    if text.is_none() {
        send!(msg, bot, "You should reply to text message");
        return Ok(());
    }
    let text = text.unwrap();

    let args = msg.text().unwrap().split(' ').skip(1).collect::<Vec<_>>();

    if args.is_empty() {
        send!(msg, bot, "You should at least give me one target language");
        return Ok(());
    }

    let mut source_lang = None;
    let target_lang;

    macro_rules! parse {
        ($str:expr) => {{
            let lang = deepl::Lang::from(&$str.to_uppercase());
            if lang.is_err() {
                send!(msg, bot, format!("invalid language code {}", args[0]));
                return Ok(());
            }
            lang.unwrap()
        }};
    }

    if args.len() == 1 {
        target_lang = parse!(args[0]);
    } else {
        source_lang = Some(parse!(args[0]));
        target_lang = parse!(args[1]);
    }

    let current_usage = rt.translator.get_usage().await;
    if let Err(e) = current_usage {
        send!(msg, bot, format!("fail to get current api usage: {e}"));
        return Ok(());
    }
    let current_usage = current_usage.unwrap();
    if current_usage.character_count > current_usage.character_limit / 3 {
        send!(msg, bot, "API usage limit are met, this command is temporary unusable.");
        return Ok(());
    }

    let tr_result = rt
        .translator
        .translate(text, source_lang, target_lang)
        .await;

    if let Err(err) = tr_result {
        send!(msg, bot, format!("fail to translate: {err}"));
    } else {
        let response = tr_result.unwrap();
        let full_text = response
            .translations
            .iter()
            .map(|rp| rp.text.as_str())
            .collect::<String>();
        send!(msg, bot, full_text);
    }

    Ok(())
}

/// handler for /pacman command
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

    send!(@Typing; msg, bot);

    use modules::provider::ArchLinuxPkgProvider;

    match operation {
        "-Si" => {
            let pkg = text.next();
            if pkg.is_none() {
                send!(msg, bot, "No package name! Abort");
                return Ok(());
            }
            let resp = rt.req.get_pkg_info(pkg.unwrap()).await;
            match resp {
                Ok(s) => send!(msg, bot, format!("{s}")),
                Err(e) => send!(msg, bot, format!("{e}")),
            }
        }
        "-Ss" => {
            let pkg = text.next();
            if pkg.is_none() {
                send!(msg, bot, "No package name! Abort");
                return Ok(());
            }
            let resp = rt.req.search_pkg(pkg.unwrap(), 8).await;
            match resp {
                Ok((Some(exact), list)) => {
                    send!(
                        msg,
                        bot,
                        format!(
                            "Found exact match: \n{}\n\n---\nResults:\n{}",
                            exact,
                            list.join("\n")
                        ),
                        html
                    )
                }
                Ok((None, list)) => send!(msg, bot, list.join("\n")),
                Err(e) => send!(msg, bot, format!("{e}")),
            }
        }
        "-Syu" => {
            if rand::random() {
                send!(
                    msg,
                    bot,
                    "Wow, you are lucky! The full system was upgraded successfully!"
                )
            } else {
                send!(msg, bot, "Oops, your system is broken during the upgrade!")
            }
        }
        _ => {
            send!(
                msg,
                bot,
                format!("This is a query bot, it doesn't support `{operation}` operation! Abort")
            )
        }
    };

    Ok(())
}

/// handler for /hitksyx command
async fn ksyx_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let mut conn = rt.cache.lock().await;
    use modules::cache::KsyxCounterCache;
    let old_v = conn.hit().await;
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

/// helper function for parsing the ehentai link
async fn parse_eh_gidlist(msg: &Message, bot: &AutoSend<Bot>) -> Result<Vec<[String; 2]>> {
    send!(@UploadPhoto; msg, bot);

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

/// handler for the /eh command
async fn eh_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    use modules::provider::EhentaiProvider;
    let gid_list = parse_eh_gidlist(&msg, &bot).await?;
    let response = rt.req.fetch_ehentai_comic_data(&gid_list).await;
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

/// handler for the /ehseed command
async fn eh_seed_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let gid_list = parse_eh_gidlist(&msg, &bot).await?;

    use modules::provider::EhentaiProvider;
    let response = rt.req.fetch_ehentai_comic_data(&gid_list).await;
    match response {
        Ok(resp) => {
            if resp.gmetadata.is_empty() {
                bot.send_message(msg.chat.id, "invalid eh link").await?;
                return Ok(());
            }

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

/// handler for the /mjx command
async fn mjx_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    use modules::provider::NsfwProvider;
    send!(@UploadPhoto; msg, bot);

    let resp = rt.req.fetch_photograph().await;

    match resp {
        Ok(s) => {
            bot.send_photo(msg.chat.id, teloxide::types::InputFile::url(s))
                .await?
        }
        Err(e) => bot.send_message(msg.chat.id, e.to_string()).await?,
    };

    Ok(())
}

/// handler for the /cookpiggy command
async fn cook_piggy_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    use modules::provider::RecipeProvider;

    let recipe = rt.req.get_pig_recipe().await;
    if let Err(e) = recipe {
        send!(msg, bot, format!("今天没法吃 piggy 了呜呜呜: {e}"));
        return Ok(());
    }

    send!(msg, bot, recipe.unwrap());

    Ok(())
}

/// handler for the /ghs command
async fn ghs_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    use modules::provider::NsfwProvider;

    send!(@UploadPhoto; msg, bot);

    let resp = rt.req.fetch_anime_image().await;

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

/// handler for the collect command
async fn collect_handler(msg: Message, bot: AutoSend<Bot>, dialogue: Dialogue) -> Result<()> {
    send!(
        msg,
        bot,
        "你可以开始转发信息了，使用命令 /collectdone 来结束命令收集"
    );
    dialogue.update(DialogueStatus::CmdCollectRunning).await?;
    Ok(())
}

/// message handler for the /collect command
async fn collect_message(msg: Message, rt: RedisRT) -> Result<()> {
    let mut collector = rt.cache.lock().await;
    let who_want_these = msg
        .from()
        .expect("Unexpectedly add non-user into dialogue")
        .id
        .0;

    let msg_from = {
        if let Some(original_sender) = msg.forward_from_user() {
            original_sender.first_name.clone()
        } else if let Some(original_sender_name) = msg.forward_from_sender_name() {
            original_sender_name.to_string()
        } else {
            "Anoynomous".to_string()
        }
    };

    let msg_text = {
        if let Some(text) = msg.text() {
            text.to_string()
        } else if msg.video().is_some() {
            "[video]".to_string()
        } else if msg.audio().is_some() {
            "[audio]".to_string()
        } else if msg.sticker().is_some() {
            "[sticker]".to_string()
        } else if msg.photo().is_some() {
            "[photo]".to_string()
        } else {
            "Unsupported message type".to_string()
        }
    };

    use crate::modules::{cache::CollectedMsgCache, prelude::MsgForm};
    collector
        .push(who_want_these, MsgForm::new(msg_from, msg_text))
        .await?;
    Ok(())
}

/// handler for /collectdone command
async fn exit_collect_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    dialogue: Dialogue,
    rt: RedisRT,
) -> Result<()> {
    use modules::cache::CollectedMsgCache;

    let msg_id = bot
        .send_message(msg.chat.id, "收集完毕，正在处理信息...")
        .await?
        .id;
    dialogue.exit().await?;

    let mut collector = rt.cache.lock().await;

    // FIXME: Can I guarantee that command must came from a user?
    let result = collector
        .finish(msg.from().expect("Message came from non-user").id.0)
        .await;
    match result {
        Some(s) if s.is_empty() => {
            bot.edit_message_text(
                msg.chat.id,
                msg_id,
                "Empty message, something is going wrong...",
            )
            .await?
        }
        Some(s) => bot.edit_message_text(msg.chat.id, msg_id, s).await?,
        None => {
            bot.edit_message_text(msg.chat.id, msg_id, "你还没有收集过消息")
                .await?
        }
    };

    Ok(())
}

/// exchange calculate helper function
async fn calculate_exchange(rt: RedisRT, amount: f64, from: String, to: String) -> Result<String> {
    use modules::cache::CurrenciesCache;
    use modules::provider::CurrenciesRateProvider;

    let mut cache = rt.cache.lock().await;
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

    let rate_info = rt.req.fetch_rate(&from, &to).await?;

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

/// handler for /exchange command
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
    use super::Command;
    use teloxide::utils::command::BotCommands;
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn get_weather(msg: Message, rt: RedisRT) -> Result<String> {
    use modules::provider::WeatherProvider;
    let text = msg.text().unwrap();
    let parts = text.split(' ').collect::<Vec<&str>>();
    if parts.len() < 2 {
        anyhow::bail!("No enough argument. Usage example: /weather 上海")
    }

    let (text, pic) = rt.req.fetch_weather(parts[1]).await?;
    Ok(format!("<a href=\"{pic}\">{text}</a>"))
}

/// command for /weather command
async fn weather_handler(msg: Message, bot: AutoSend<Bot>, rt: RedisRT) -> Result<()> {
    let chat_id = msg.chat.id;
    send!(@Typing; msg, bot);

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
