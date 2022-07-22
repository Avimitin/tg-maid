use crate::modules::runtime::Runtime;
use anyhow::Result;
use redis::aio::ConnectionManager as redis_cm;
use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::prelude::*;

use crate::modules::weather;

#[derive(Clone)]
enum DialogueStatus {
    None,
    CmdCollectRunning,
}

impl std::default::Default for DialogueStatus {
    fn default() -> Self {
        Self::None
    }
}

type Dialogue = dialogue::Dialogue<DialogueStatus, dialogue::InMemStorage<DialogueStatus>>;
type RedisRT = Runtime<redis_cm, redis_cm, weather::WttrInApi>;

pub fn handler_schema() -> UpdateHandler<anyhow::Error> {
    use crate::modules::commands::Command;

    let stateless_cmd_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::Exchange].endpoint(exchange_handler))
        .branch(dptree::case![Command::Help].endpoint(help_handler))
        .branch(dptree::case![Command::Weather].endpoint(weather_handler))
        .branch(dptree::case![Command::Collect].endpoint(collect_handler));

    let stausful_cmd_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::CollectDone].endpoint(exit_collect_handler));

    let msg_handler = Update::filter_message()
        .branch(dptree::case![DialogueStatus::None].branch(stateless_cmd_handler))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].branch(stausful_cmd_handler))
        .branch(dptree::case![DialogueStatus::CmdCollectRunning].endpoint(collect_handler));

    let root = dptree::entry().branch(msg_handler);

    dialogue::enter::<Update, dialogue::InMemStorage<DialogueStatus>, DialogueStatus, _>()
        .branch(root)
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

async fn calculate_exchange(
    msg: Message,
    rt: RedisRT,
) -> Result<String> {
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

async fn exchange_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    rt: Runtime<redis_cm, redis_cm, weather::WttrInApi>,
) -> Result<()> {
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

async fn help_handler(msg: Message, bot: AutoSend<Bot>) -> Result<()> {
    use crate::modules::commands::Command;
    use teloxide::utils::command::BotCommands;
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn get_weather(
    msg: Message,
    rt: Runtime<redis_cm, redis_cm, weather::WttrInApi>,
) -> Result<String> {
    use crate::modules::weather::WeatherFetcher;

    let text = msg.text().unwrap();
    let parts = text.split(' ').collect::<Vec<&str>>();
    if parts.len() < 2 {
        anyhow::bail!("No enough argument. Usage example: /weather 上海")
    }

    let text = rt.weather.query(parts[1]).await?;
    let pic = rt.weather.pic(parts[1]);
    Ok(format!("<a href=\"{pic}\">{text}</a>"))
}

async fn weather_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    rt: Runtime<redis_cm, redis_cm, weather::WttrInApi>,
) -> Result<()> {
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
