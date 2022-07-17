use crate::currency::{InMemCache, RateCalculator};
use anyhow::Result;
use std::sync::Arc;
use teloxide::{prelude::*, utils::command::BotCommands};

// FIXME: This mutex locker is not suggested to be used
// But vaultwarden used it in their projects.
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
struct Runtime {
    currency: Arc<Mutex<RateCalculator<InMemCache>>>,
}

impl Runtime {
    pub fn new() -> Self {
        let cache = InMemCache::new();
        Self {
            currency: Arc::new(Mutex::new(RateCalculator::new(cache))),
        }
    }
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {

    #[command(description = "Display this help message")]
    Help,

    #[command(description = "Search exchange rate. Usage example: /exchange 1 usd cny")]
    Exchange,

    #[command(
        description = "Search weather. Usage example: /weather 上海",
        parse_with = "split"
    )]
    Weather { city: String },

    #[command(description = "获取买家秀")]
    Mjx,

    #[command(description = "随机二次元色图")]
    Ghs,

    #[command(description = "查询 e-hentai 链接内的本子信息", parse_with = "split")]
    Eh { url: String },

    #[command(description = "收集所有内容并合并")]
    Collect,

    #[command(description = "Search package information in Arch Linux Repo and AUR")]
    Pacman,

    #[command(description = "Interact with ksyx")]
    HitKsyx,

    #[command(description = "Interact with piggy")]
    CookPiggy,
}

async fn calculate_exchange(msg: Message, rt: Runtime) -> Result<String> {
    let text = msg
        .text()
        .ok_or_else(|| anyhow::anyhow!("Can not process empty text message"))?;

    let args = text.split(' ').collect::<Vec<&str>>();
    if args.len() < 4 {
        anyhow::bail!("No enough arguments")
    }

    let amount = args[1]
        .parse::<f64>()
        .map_err(|e| anyhow::anyhow!("fail to parse {} into integer: {e}", args[1]))?;
    let (from, to) = (args[2].to_lowercase(), args[3].to_lowercase());
    let mut calculator = rt.currency.lock().await;
    let (result, date) = calculator.calc(amount, &from, &to).await?;

    Ok(format!(
        r#"
{} ({}) --> {} ({})

{} ==> {}

Date: {}
               "#,
        from.to_uppercase(),
        calculator.get_fullname(&from).await.unwrap(),
        to.to_uppercase(),
        calculator.get_fullname(&to).await.unwrap(),
        amount,
        result,
        date
    ))
}

async fn cmd_exchange(msg: Message, bot: AutoSend<Bot>, rt: Runtime) -> Result<()> {
    let chat_id = msg.chat.id;

    let callback = bot.send_message(chat_id, "Fetching API...").await?;

    match calculate_exchange(msg, rt).await {
        Ok(reply) => bot.edit_message_text(chat_id, callback.id, reply).await?,
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

async fn cmd_help(msg: Message, bot: AutoSend<Bot>) -> Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

pub async fn run() {
    let commands_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::Exchange].endpoint(cmd_exchange))
        .branch(dptree::case![Command::Help].endpoint(cmd_help));

    let handler = Update::filter_message().branch(commands_handler);

    let bot = Bot::from_env().auto_send();
    let runtime = Runtime::new();
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![runtime])
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}
