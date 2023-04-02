use redis::Commands;
use rosu_v2::{
    prelude::{EventType, RecentEvent},
    request::UserId,
};
use teloxide::{
    payloads::SendMessageSetters,
    requests::Requester,
    types::{ChatId, ParseMode},
};

use crate::{
    app::AppData,
    event::EventWatcher,
    helper::{self, Html},
};

use super::Sendable;

pub async fn notify_user_latest_event(
    data: AppData,
    uid: impl Into<UserId>,
) -> anyhow::Result<Sendable> {
    let unreported = get_user_recent_event(&data, uid).await?;
    if unreported.is_empty() {
        return Ok(Sendable::text("No more new event"));
    }
    let notification = unreported.into_iter().fold(String::new(), |accum, elem| {
        format!("{accum}\n\n* {}", format_event_type(elem.event_type))
    });

    Ok(Sendable::Text(notification))
}

pub fn spawn_osu_user_event_watcher(bot: teloxide::Bot, data: AppData) {
    let state = OsuUserEventState {
        user_ids: helper::env_get_var("OSU_USER_IDS")
            .split(',')
            .map(|id| id.to_string())
            .collect(),
        groups: helper::get_list_from_env("OSU_USER_EVENT_NOTIFY_GROUP"),
    };

    let event_watcher = EventWatcher::builder()
        .bot(bot)
        .data(data)
        .state(state)
        .build();

    event_watcher.start("Osu Event Watcher", 60 * 5, watch);
}

async fn watch(ctx: EventWatcher<OsuUserEventState>) -> anyhow::Result<()> {
    for user in &ctx.state.user_ids {
        let unreported = get_user_recent_event(&ctx.data, user).await?;
        if unreported.is_empty() {
            return Ok(());
        }

        let notification = unreported.into_iter().fold(String::new(), |accum, elem| {
            format!("{accum}\n\n* {}", format_event_type(elem.event_type))
        });

        for chat in &ctx.state.groups {
            ctx.bot
                .send_message(ChatId(*chat), notification.as_str())
                .parse_mode(ParseMode::Html)
                .await?;
        }
    }

    Ok(())
}

struct OsuUserEventState {
    user_ids: Vec<String>,
    groups: Vec<i64>,
}

macro_rules! osu_url {
    ($suffix:expr) => {
        format!("https://osu.ppy.sh{}", $suffix)
    };
}

fn format_event_type(typ: EventType) -> String {
    match typ {
        EventType::Rank {
            grade,
            rank,
            beatmap,
            user,
            ..
        } => format!(
            "User {} get rank {} on {} with grade {}",
            Html::a(&osu_url!(user.url), &user.username),
            Html::b(rank),
            Html::a(&osu_url!(beatmap.url), &beatmap.title),
            Html::b(grade)
        ),
        EventType::Medal { medal, user } => {
            format!(
                "User {} get medal [{}]({})",
                user.username, medal.name, medal.description
            )
        }
        EventType::UsernameChange { user } => {
            format!(
                "User {} update username to {}",
                user.previous_username.unwrap(),
                user.username
            )
        }
        EventType::SupportAgain { user } | EventType::SupportFirst { user } => {
            format!("User {} is now supporting osu!", Html::b(user.username))
        }
        _ => format!("New event: {:?}", typ),
    }
}

async fn get_user_recent_event(
    data: &AppData,
    uid: impl Into<UserId>,
) -> anyhow::Result<Vec<RecentEvent>> {
    let uid: UserId = uid.into();
    let events = data.osu.recent_events(uid.clone()).await?;
    if events.is_empty() {
        return Ok(events);
    }

    let unreported: &[RecentEvent];

    let last_offset = get_last_event_offset(data, &uid)?;
    if let Some(last_offset) = last_offset {
        let p = events.partition_point(|event| event.created_at.unix_timestamp() > last_offset);
        if p == 0 {
            return Ok(Vec::new());
        }
        unreported = &events[0..p];
    } else {
        unreported = events.as_slice();
    }

    cache_event_offset(data, &uid, &unreported[0])?;

    Ok(unreported.to_vec())
}

fn get_last_event_offset(data: &AppData, user: &UserId) -> anyhow::Result<Option<i64>> {
    let key = format!("OSU_EVENT_OFFSET:{}", user);
    let data: Option<i64> = data.cacher.get_conn().get(key)?;
    Ok(data)
}

fn cache_event_offset(data: &AppData, user: &UserId, event: &RecentEvent) -> anyhow::Result<()> {
    let key = format!("OSU_EVENT_OFFSET:{}", user);
    let val = event.created_at.unix_timestamp();

    data.cacher.get_conn().set(key, val)?;

    Ok(())
}

#[tokio::test]
async fn test_get_user_activity() {
    dotenv::dotenv().ok();

    let client_id: u64 = crate::helper::parse_from_env("OSU_CLIENT_ID");
    let client_secret = crate::helper::env_get_var("OSU_CLIENT_SECRET");

    let osu = rosu_v2::Osu::new(client_id, client_secret)
        .await
        .unwrap_or_else(|err| panic!("fail to create osu client: {err}"));

    let events = osu.recent_events(16900842).await.unwrap();
    dbg!(events);
}
