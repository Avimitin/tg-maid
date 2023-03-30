use anyhow::Result;
use rand::Rng;
use teloxide::{
    dispatching::{dialogue, UpdateHandler},
    payloads::SendPhotoSetters,
    prelude::*,
    types::ParseMode,
    utils::command::BotCommands,
};

use rusty_maid::{
    app::AppData,
    modules::{self, Sendable},
    sendable,
};

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
macro_rules! generate_commands {
    (
        stateless: {
            $(
                #[desc = $desc:literal]
                $cmd:ident,
            )+
        }
        stateful: {
            $(
                #[desc = $sdesc:literal]
                $scmd:ident,
            )+
        }
    ) => {
        #[derive(BotCommands, Clone, Debug)]
        #[command(
            rename_rule = "lowercase",
            description = "These commands are supported:"
        )]
        pub enum Command {
            $(
                #[command(description=$desc)]
                $cmd,
            )+
            $(
                #[command(description=$sdesc)]
                $scmd,
            )+
        }

        paste::paste! {
            fn generate_stateless_cmd_handler() -> UpdateHandler<anyhow::Error>  {
                teloxide::filter_command::<Command, _>()
                    $(
                        .branch(
                            dptree::case![Command::$cmd]
                                .endpoint( [< $cmd:snake _handler>] )
                        )
                    )+
            }
        }
    };
}

generate_commands! {
    stateless: {
        #[desc = "Display this help message"]
        Help,
        #[desc = "Search weather. Usage example: /weather 上海"]
        Weather,
        #[desc =  "Search exchange rate. Usage example: /exchange 1 usd cny"]
        Exchange,
        #[desc = "随机二次元色图"]
        Ghs,
        #[desc = "查询 e-hentai 链接内的本子信息"]
        Eh,
        #[desc = "收集所有信息并合并"]
        Collect,
        #[desc = "Search package information in Arch Linux Repo and AUR"]
        Pacman,
        #[desc = "Interact with ksyx"]
        HitKsyx,
        #[desc = "Interact with piggy"]
        CookPiggy,
        #[desc = "Get some useful id"]
        Id,
        #[desc = "Translate text by DeepL"]
        Tr,
    }
    stateful: {
        #[desc = "Finish Collect"]
        CollectDone,
    }
}

macro_rules! send_action {
    (@$action:ident; $msg:ident, $bot:ident) => {
        $bot.send_chat_action($msg.chat.id, teloxide::types::ChatAction::$action)
            .await?
    };
}

macro_rules! abort {
    ($bot:expr, $msg:expr, $($arg:tt)*) => {
        $bot.send_message($msg.chat.id, format!($($arg)*)).await?;
        return Ok(());
    };
}

macro_rules! handle_result {
    ($bot:expr, $msg:expr, $result:expr, $on_failure:literal) => {
        match $result {
            Ok(sendable) => {
                sendable.send(&$bot, &$msg).await?;
            }
            Err(err) => {
                abort!($bot, $msg, "{}: {}", $on_failure, err);
            }
        }
    };
}

pub fn handler_schema() -> UpdateHandler<anyhow::Error> {
    let stateless_cmd_handler = generate_stateless_cmd_handler();

    let stateful_cmd_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::CollectDone].endpoint(collect_done_handler));

    let msg_handler = Update::filter_message()
        .branch(dptree::case![DialogueStatus::None].branch(stateless_cmd_handler))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].branch(stateful_cmd_handler))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].endpoint(collect_message_handler));

    let root = dptree::entry().branch(msg_handler);

    dialogue::enter::<Update, dialogue::InMemStorage<DialogueStatus>, DialogueStatus, _>()
        .branch(root)
}

async fn help_handler(msg: Message, bot: Bot) -> Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn weather_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    send_action!(@Typing; msg, bot);

    let text = msg.text().unwrap();
    let parts = text.split(' ').collect::<Vec<&str>>();
    if parts.len() < 2 {
        abort!(bot, msg, "No enough argument. Usage: /weather 上海");
    }

    let result = modules::weather::fetch_weather(data, text).await;

    handle_result!(bot, msg, result, "fail to get weather");

    Ok(())
}

async fn exchange_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    send_action!(@Typing; msg, bot);

    let text = msg.text().unwrap();
    let parts = text.split(' ').collect::<Vec<&str>>();
    if parts.len() < 4 {
        abort!(bot, msg, "No enough argument. Usage: /exchange 123 JPY CNY");
    }

    let Ok(amount) = parts[1].parse::<f64>() else {
        abort!(bot, msg, "Not a valid number: {}", parts[1]);
    };

    let result = modules::currency::exchange(
        data,
        amount,
        &parts[2].to_lowercase(),
        &parts[3].to_lowercase(),
    )
    .await;

    match result {
        Ok(sendable) => {
            sendable!(bot, msg, sendable, format = Html);
        }
        Err(err) => {
            abort!(bot, msg, "{}: {}", "fail to make currency exchange", err);
        }
    };

    Ok(())
}

async fn ghs_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    send_action!(@UploadPhoto; msg, bot);

    let result = modules::nsfw::fetch_nsfw_anime_img(data).await;
    match result {
        Ok(sendable) => {
            let Sendable::File(file, caption) = sendable else {
                panic!("Bad Implementation")
            };
            bot.send_photo(msg.chat.id, file)
                .parse_mode(ParseMode::Html)
                .caption(caption.unwrap())
                .has_spoiler(true)
                .await?;
        }
        Err(err) => {
            abort!(bot, msg, "{}: {}", "fail to get image", err);
        }
    };

    Ok(())
}

fn get_args(msg: &Message) -> anyhow::Result<String> {
    let text = msg.text().unwrap();
    if let Some(args) = text.split_once(' ') {
        Ok(args.1.to_string())
    } else if let Some(reply_to) = msg.reply_to_message() {
        let text = reply_to.text();
        if text.is_none() {
            anyhow::bail!("You need to reply to a text message");
        }

        Ok(text.unwrap().to_string())
    } else {
        anyhow::bail!("You need to attach text after the command, or reply to a text message")
    }
}

async fn eh_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    send_action!(@UploadPhoto; msg, bot);

    let args = get_args(&msg);
    if let Err(err) = args {
        abort!(bot, msg, "{}", err);
    }

    let args = args.unwrap();
    let parse_result = modules::ehentai::parse_gid_list(&args);

    if let Err(err) = parse_result {
        abort!(bot, msg, "{}", err);
    }

    let result = modules::ehentai::fetch_ehentai_comic_data(data, parse_result.unwrap()).await;
    match result {
        Ok(sendables) => {
            for s in sendables {
                s.send(&bot, &msg).await?;
            }
        }
        Err(err) => {
            abort!(bot, msg, "fail to get ehentai data: {}", err);
        }
    }

    Ok(())
}

async fn cook_piggy_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    send_action!(@Typing; msg, bot);

    let recipe = modules::piggy::get_pig_recipe(data).await;
    handle_result!(bot, msg, recipe, "fail to cook piggy");

    Ok(())
}

/// handler for the collect command
async fn collect_handler(msg: Message, bot: Bot, dialogue: Dialogue) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "你可以开始转发信息了，使用命令 /collectdone 来结束命令收集",
    )
    .await?;
    dialogue.update(DialogueStatus::CmdCollectRunning).await?;
    Ok(())
}

async fn collect_message_handler(
    bot: Bot,
    msg: Message,
    data: AppData,
    dialogue: Dialogue,
) -> Result<()> {
    let chat_id = msg.chat.id;
    if let Err(err) = modules::collect::push_msg(data, msg).await {
        bot.send_message(chat_id, format!("fail to collect message: {err}"))
            .await?;
        dialogue.exit().await?;
    };
    Ok(())
}

async fn collect_done_handler(
    msg: Message,
    bot: Bot,
    dialogue: Dialogue,
    data: AppData,
) -> Result<()> {
    let msg = bot
        .send_message(msg.chat.id, "Collect done, transforming...")
        .await?;
    dialogue.exit().await?;

    let result = modules::collect::finish(data, &msg).await;
    handle_result!(bot, msg, result, "fail to collect message");
    Ok(())
}

async fn tr_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    const HELP: &str = "\
            Usage: Reply to a text message and input: \n\
                \t/tr [source language(optional)] target-language.\n\
            Example:\n\
                \t/tr zh en\
            ";
    let replyto = msg.reply_to_message();
    if replyto.is_none() {
        abort!(bot, msg, "You should reply to a text message. \n{}", HELP);
    }

    let text = replyto.unwrap().text();
    if text.is_none() {
        abort!(bot, msg, "You should reply to a text message. \n{}", HELP);
    }
    let text = text.unwrap();

    let args = msg.text().unwrap().split(' ').skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        abort!(
            bot,
            msg,
            "You should at least give me a target language. \n{}",
            HELP
        );
    }

    let mut source_lang = None;
    let target_lang;

    macro_rules! parse_lang {
        ($str:expr) => {{
            let lang = deepl::Lang::try_from(&$str.to_uppercase());
            if lang.is_err() {
                abort!(bot, msg, "invalid language code {}", args[0]);
            }
            lang.unwrap()
        }};
    }

    if args.len() == 1 {
        target_lang = parse_lang!(args[0]);
    } else {
        source_lang = Some(parse_lang!(args[0]));
        target_lang = parse_lang!(args[1]);
    }

    let current_usage = data.deepl.get_usage().await;
    if let Err(e) = current_usage {
        abort!(bot, msg, "fail to get current api usage: {}", e);
    }
    let current_usage = current_usage.unwrap();
    if current_usage.character_count > current_usage.character_limit / 3 {
        abort!(
            bot,
            msg,
            "API usage limit are met, this command is temporary unusable."
        );
    }

    let result = if let Some(src) = source_lang {
        data.deepl
            .translate_text(text, target_lang)
            .source_lang(src)
            .await
    } else {
        data.deepl.translate_text(text, target_lang).await
    };

    if let Err(err) = result {
        abort!(bot, msg, "fail to translate: {}", err);
    }

    let resp = result.unwrap();
    let full_text = resp
        .translations
        .iter()
        .map(|rp| rp.text.as_str())
        .collect::<String>();
    bot.send_message(msg.chat.id, full_text).await?;

    Ok(())
}

async fn pacman_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    let mut text = msg.text().unwrap().split(' ');
    // shift one
    text.next();

    let operation = text.next();
    if operation.is_none() {
        abort!(bot, msg, "No operation was given, abort!");
    }
    let operation = operation.unwrap();

    send_action!(@Typing; msg, bot);

    match operation {
        "-Si" => {
            let pkg = text.next();
            if pkg.is_none() {
                abort!(bot, msg, "No package name! Abort");
            }
            let resp = modules::archlinux::fetch_pkg_info(data, pkg.unwrap()).await;
            handle_result!(bot, msg, resp, "fail to get pkg info");
        }
        "-Ss" => {
            let pkg = text.next();
            if pkg.is_none() {
                abort!(bot, msg, "No package name! Abort");
            }
            let resp = modules::archlinux::fetch_pkg_list(data, pkg.unwrap(), 8).await;
            handle_result!(bot, msg, resp, "fail to get pkg");
        }
        "-Syu" => {
            if rand::random() {
                bot.send_message(
                    msg.chat.id,
                    "Wow, you are lucky! The full system was upgraded successfully!",
                )
                .await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Oops, your system is broken during the upgrade!",
                )
                .await?;
            }
        }
        _ => {
            bot.send_message(msg.chat.id, "Unimplemented").await?;
        }
    };

    Ok(())
}

async fn hit_ksyx_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    let old = modules::ksyx::hit(data);
    if let Err(ref e) = old {
        abort!(bot, msg, "fail to interact with ksyx: {}", e);
    }

    let action = &[
        "爱抚", "中出", "暴打", "后入", "膜", "贴贴", "狂踹", "寸止", "绳缚",
    ];

    let choice = rand::thread_rng().gen_range(0..action.len());
    bot.send_message(
        msg.chat.id,
        format!(
            "{} {}了 ksyx，ksyx 已经被动手动脚了 {} 次",
            msg.from().unwrap().first_name,
            action[choice],
            old.unwrap(),
        ),
    )
    .await?;

    Ok(())
}

async fn id_handler(msg: Message, bot: Bot) -> Result<()> {
    let user_id = if let Some(reply) = msg.reply_to_message() {
        reply.from().map_or(0, |user| user.id.0)
    } else {
        msg.from().map_or(0, |user| user.id.0)
    };
    let chat_id = msg.chat.id;

    bot.send_message(
        msg.chat.id,
        format!("user id: {user_id}\nchat id: {chat_id}"),
    )
    .await?;

    Ok(())
}
