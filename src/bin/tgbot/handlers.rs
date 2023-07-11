use anyhow::Result;
use image::ImageFormat;
use rand::Rng;
use redis::Commands;
use teloxide::{
    dispatching::{dialogue, UpdateHandler},
    net::Download,
    payloads::SendPhotoSetters,
    prelude::*,
    types::{
        ChatKind, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, InputSticker, ParseMode,
        User,
    },
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
        #[desc = "Get event from someone"]
        OsuEvent,
        #[desc = "Roll a number"]
        Roll,
        #[desc = "Make a image to record somebody's quote"]
        MakeQuote,
        #[desc = "Delete a sticker create by this bot"]
        DelSticker,
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
                abort!($bot, $msg, "{}: {:?}", $on_failure, err);
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

    let callback_handler = Update::filter_callback_query().endpoint(callback_dispatcher);

    let root = dptree::entry().branch(msg_handler).branch(callback_handler);

    dialogue::enter::<Update, dialogue::InMemStorage<DialogueStatus>, DialogueStatus, _>()
        .branch(root)
}

async fn callback_dispatcher(cb: CallbackQuery, bot: Bot, app_data: AppData) -> anyhow::Result<()> {
    bot.answer_callback_query(&cb.id).await?;

    if cb.data.is_none() || cb.message.is_none() {
        return Ok(());
    }

    let cb_data = cb.data.as_deref().unwrap();
    match cb_data {
        "sticker.make_quote.from_photo" => {
            add_photo_from_msg_to_sticker_set(cb, bot, app_data).await?
        }
        _ => return Ok(()),
    }

    Ok(())
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
            sendable!(bot, msg, sendable, format = Html, spoiler = on);
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
                sendable!(bot, msg, s, format = Html, spoiler = on);
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
    if let teloxide::types::ChatKind::Public(_) = msg.chat.kind {
        abort!(bot, msg, "This command can only be used in private chat");
    }
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
    send_action!(@Typing; msg, bot);
    dialogue.exit().await?;

    let result = modules::collect::finish(data, &msg).await;
    match result {
        Ok(sendable) => {
            sendable!(bot, msg, sendable, format = Html);
        }
        Err(err) => {
            abort!(bot, msg, "{}: {}", "fail to collect message", err);
        }
    };
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
        abort!(bot, msg, "fail to translate: {:?}", err);
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
            match resp {
                Ok(sendable) => {
                    sendable!(bot, msg, sendable, format = Html);
                }
                Err(err) => {
                    abort!(bot, msg, "{}: {:?}", "fail to get pkg info", err);
                }
            };
        }
        "-Ss" => {
            let pkg = text.next();
            if pkg.is_none() {
                abort!(bot, msg, "No package name! Abort");
            }
            let resp = modules::archlinux::fetch_pkg_list(data, pkg.unwrap(), 8).await;
            match resp {
                Ok(sendable) => {
                    sendable!(bot, msg, sendable, format = Html);
                }
                Err(err) => {
                    abort!(bot, msg, "{}: {:?}", "fail to get pkg", err);
                }
            };
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

async fn osu_event_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    send_action!(@Typing; msg, bot);

    let text = msg.text().unwrap();
    let parts = text.split_once(' ');

    let Some((_, username)) = parts else {
        abort!(bot, msg, "No enough argument. Usage: /osu_event username");
    };

    let result = modules::osu::notify_user_latest_event(data, username).await;
    match result {
        Ok(sendable) => {
            sendable!(bot, msg, sendable, format = Html);
        }
        Err(err) => {
            abort!(
                bot,
                msg,
                "fail to get last event for user {}: {:?}",
                username,
                err
            );
        }
    }

    Ok(())
}

async fn roll_handler(msg: Message, bot: Bot) -> Result<()> {
    send_action!(@Typing; msg, bot);

    let text = msg.text().unwrap();
    let parts = text.split_once(' ');

    let choosen: u64;
    if let Some((_, max)) = parts {
        let Ok(max) = max.parse::<u64>() else {
            abort!(bot, msg, "expect number");
        };

        choosen = rand::thread_rng().gen_range(0..max);
    } else {
        choosen = rand::random();
    }

    bot.send_message(msg.chat.id, choosen.to_string()).await?;

    Ok(())
}

fn create_quote_from_username(
    target: &User,
    username: &str,
    quote: &str,
    data: &AppData,
) -> anyhow::Result<InputFile> {
    let avatar = make_quote::SpooledData::TgRandom {
        id: target.id.0,
        name: target.first_name.to_string(),
    };
    let quote_config = make_quote::ImgConfig::builder()
        .username(username)
        .quote(format!("「{}」", quote))
        .avatar(&avatar)
        .build();
    let result = data.quote_maker.make_image(&quote_config)?;
    Ok(InputFile::memory(result))
}

async fn create_quote(
    bot: &Bot,
    target: &User,
    quote: &str,
    data: &AppData,
) -> anyhow::Result<InputFile> {
    let photos = bot
        .get_user_profile_photos(target.id)
        .limit(1)
        .await?
        .photos;

    let username = if let Some(username) = &target.username {
        format!("@{username}")
    } else {
        format!("- {}", target.first_name)
    };

    if photos.is_empty() || photos[0].is_empty() {
        let img = create_quote_from_username(target, &username, quote, data)?;
        return Ok(img);
    }

    let avatar_id = &photos[0]
        .iter()
        .max_by(|x, y| x.width.cmp(&y.width))
        .unwrap()
        .file
        .id;
    let file = bot.get_file(avatar_id).await?;
    let avatar_cacher_key = format!("TG_AVATAR:USER:{}", avatar_id);
    let cache: Option<Vec<u8>> = data.cacher.get_conn().get(&avatar_cacher_key)?;

    let avatar = if let Some(cache) = cache {
        cache
    } else {
        let mut avatar = std::io::Cursor::new(Vec::with_capacity(file.size as usize));
        bot.download_file(&file.path, &mut avatar).await?;
        avatar.into_inner()
    };

    data.cacher
        .get_conn()
        .set_ex(avatar_cacher_key, avatar.as_slice(), 60 * 60 * 24)?;

    let quote_config = make_quote::ImgConfig::builder()
        .username(username)
        .quote(format!("「{}」", quote))
        .avatar(avatar.as_slice())
        .build();
    let result = data.quote_maker.make_image(&quote_config)?;
    Ok(InputFile::memory(result))
}

async fn make_quote_handler(msg: Message, bot: Bot, data: AppData) -> Result<()> {
    send_action!(@Typing; msg, bot);
    let Some(reply_to_msg) = msg.reply_to_message() else {
        abort!(bot, msg, "You should reply to somebody's text message to generate the quote image");
    };

    let quote = if let Some(quote) = reply_to_msg.text() {
        quote
    } else {
        let Some(quote) = reply_to_msg.caption() else {
            abort!(bot, msg, "You should reply to somebody's text message to generate the quote image");
        };
        quote
    };

    let Some(target) = reply_to_msg.from() else {
        abort!(bot, msg, "You should reply to normal user");
    };

    let photo = create_quote(&bot, target, quote, &data).await?;

    send_action!(@UploadPhoto; msg, bot);

    let button = InlineKeyboardButton::callback("加入表情包", "sticker.make_quote.from_photo");
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);
    bot.send_photo(msg.chat.id, photo)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

fn unwrap_chat_name(msg: &Message) -> Result<&str, &'static str> {
    let name = if let Some(name) = msg.chat.username() {
        name
    } else if let Some(name) = msg.chat.first_name() {
        name
    } else if let Some(name) = msg.chat.last_name() {
        name
    } else if let Some(name) = msg.chat.title() {
        name
    } else {
        return Err("Action require chat username or first/last name or title");
    };

    Ok(name)
}

async fn get_chat_owner_from_cb(cb: &CallbackQuery, bot: Bot) -> Option<User> {
    let msg = cb.message.as_ref()?;
    match msg.chat.kind {
        ChatKind::Public(_) => {
            let member = bot
                .get_chat_administrators(msg.chat.id)
                .await
                .ok()?
                .into_iter()
                .find(|member| member.is_owner())?;
            Some(member.user)
        }
        ChatKind::Private(_) => Some(cb.from.clone()),
    }
}

async fn download_photo(msg: &Message, bot: Bot) -> anyhow::Result<String> {
    let photos = msg
        .photo()
        .ok_or_else(|| anyhow::anyhow!("This message doesn't contain any photo"))?;
    let file_id = photos
        .iter()
        .max_by(|x, y| x.width.cmp(&y.width))
        .unwrap_or_else(|| panic!("Fail to find any of the photo to compare? This is weird"))
        .file
        .id
        .to_string();
    let file = bot.get_file(&file_id).await?;
    // Get the file extension. It should be ".jpg", but unwrapping it from the download filename is
    // more reliable.
    let path = std::path::Path::new(&file.path).extension().unwrap();

    let dl_path = format!("/tmp/telegram-tmpfile-{file_id}.{}", path.to_string_lossy());
    let mut tmpfile = tokio::fs::File::create(&dl_path).await?;
    bot.download_file(&file.path, &mut tmpfile).await?;
    Ok(dl_path)
}

fn legalize_sticker_img(path: &str) -> anyhow::Result<()> {
    // Get the fd
    let image = image::open(path)?;
    // Tokio::fs::File doesn't implement std::io::Seek, so we need to use the std::fs::File.
    // Truncate again into same path
    let mut tmpfile = std::fs::File::create(path).unwrap();
    // Telegram doesn't accept JPG format, so we need to convert it into PNG format here.
    image
        .thumbnail(512, 512)
        .write_to(&mut tmpfile, ImageFormat::Png)?;
    Ok(())
}

async fn add_or_create_sticker_set(
    bot: Bot,
    sticker: InputSticker,
    sticker_owner: UserId,
    sticker_name: &str,
    sticker_title: &str,
) -> anyhow::Result<()> {
    let sticker_set = bot.get_sticker_set(sticker_name).await;
    if let Ok(sticker_set) = sticker_set {
        bot.add_sticker_to_set(sticker_owner, sticker_set.name, sticker, "💭")
            .await?;
    } else {
        bot.create_new_sticker_set(sticker_owner, sticker_name, sticker_title, sticker, "💭")
            .await?;
    }
    Ok(())
}

async fn add_photo_from_msg_to_sticker_set(
    cb: CallbackQuery,
    bot: Bot,
    data: AppData,
) -> anyhow::Result<()> {
    // Bound check is done by callback_dispatcher
    let msg = cb.message.as_ref().unwrap();
    let Some(keyboard) = msg.reply_markup() else {
        // Actually this should be unreachable
        abort!(bot, msg, "This photo is already added.");
    };

    let lock_key = format!("quote_sticker_set_locker:{}", msg.id);
    let mut redis_cli = data.cacher.get_conn();
    let is_lock: bool = redis_cli.get(&lock_key)?;
    if is_lock {
        return Ok(());
    } else {
        redis_cli.set_ex(&lock_key, 1, 60)?;
    }

    bot.edit_message_caption(msg.chat.id, msg.id)
        .caption("Processing image...")
        .await?;

    let result: anyhow::Result<()> = (async {
        // STEP1: Get photo file from telegram
        let dl_path = download_photo(msg, bot.clone()).await?;

        // STEP2: Resize the image to 512px
        //
        // Using operation from std::fs will probably block the whole tokio task scheduler.
        // SO I wrapped them into the `block_in_place` function to avoid that case.
        tokio::task::block_in_place(|| legalize_sticker_img(&dl_path))?;

        // STEP3: Read the resized image and send it to telegram
        bot.edit_message_caption(msg.chat.id, msg.id)
            .caption("Image converted, sending...")
            .await?;
        let sticker = InputSticker::Png(InputFile::file(&dl_path));

        // STEP4: Set the sticker
        let bot_info = bot.get_me().await?;
        let Some(sticker_owner) = get_chat_owner_from_cb(&cb, bot.clone()).await else {
            abort!(bot, msg, "Fail to find chat owner, sticker set need at least one owner");
        };
        let sticker_name = format!("quote_img_{}_by_{}", sticker_owner.id, bot_info.username());
        let chat_name = unwrap_chat_name(msg);
        if let Err(err) = chat_name {
            abort!(bot, msg, "{err}");
        }
        let sticker_title = format!("Quotes From {}", chat_name.unwrap());

        add_or_create_sticker_set(
            bot.clone(),
            sticker,
            sticker_owner.id,
            &sticker_name,
            &sticker_title,
        )
        .await?;

        // Step5: Clean up
        let sticker_set_link = rusty_maid::helper::Html::a(
            &format!("https://t.me/addstickers/{}", sticker_name),
            "sticker set",
        );
        bot.edit_message_caption(msg.chat.id, msg.id)
            .caption(format!("Image converted, see {}.", sticker_set_link))
            .parse_mode(ParseMode::Html)
            .await?;

        if let Err(err) = tokio::fs::remove_file(dl_path).await {
            abort!(
                bot,
                msg,
                "fail to remove temp file after sticker converted: {err}"
            );
        }

        Ok(())
    })
    .await;

    if let Err(err) = result {
        bot.edit_message_caption(msg.chat.id, msg.id)
            .caption(format!("Fail to convert this image into sticker: {err}"))
            .reply_markup(keyboard.clone())
            .await?;
        redis_cli.del(lock_key)?;
    }

    Ok(())
}

async fn del_sticker_handler(msg: Message, bot: Bot) -> anyhow::Result<()> {
    let Some(target_sticker_msg) = msg.reply_to_message() else {
        abort!(bot, msg, "please reply to a sticker message");
    };

    let Some(sticker) = target_sticker_msg.sticker() else {
        abort!(bot, msg, "Please reply to a sticker message");
    };

    let result = bot.delete_sticker_from_set(&sticker.file.id).await;
    if let Err(err) = result {
        abort!(bot, msg, "Fail to delete this sticker: {err}");
    }

    bot.send_message(msg.chat.id, "Deleted").await?;

    Ok(())
}
