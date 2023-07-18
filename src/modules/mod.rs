// Provider Module
pub mod archlinux;
pub mod bilibili;
pub mod collect;
pub mod currency;
pub mod ehentai;
pub mod health;
pub mod ksyx;
pub mod nsfw;
pub mod osu;
pub mod piggy;
pub mod steam;
pub mod weather;
pub mod ytd;

// Every module should provide a function that turn user input to [`Sendable`]
use reqwest::IntoUrl;
use std::fmt::Display;
use teloxide::{
    payloads::SendPhotoSetters,
    prelude::{Bot, Message, Requester},
    types::InputFile,
};

pub enum Sendable {
    Text(String),
    File(InputFile, Option<String>),
}

#[macro_export]
macro_rules! sendable {
    ($bot:expr, $msg:expr, $sendable:expr) => {
        match $sendable {
            Sendable::Text(msg) => {
                $bot.send_message($msg.chat.id, msg).await?;
            }
            Sendable::File(file, caption) => {
                if let Some(caption) = caption {
                    $bot.send_photo($msg.chat.id, file).caption(caption).await?;
                } else {
                    $bot.send_photo($msg.chat.id, file).await?;
                }
            }
        }
    };

    ($bot:expr, $msg:expr, $sendable:expr, format=$format:ident) => {
        match $sendable {
            Sendable::Text(msg) => {
                $bot.send_message($msg.chat.id, msg)
                    .parse_mode(ParseMode::$format)
                    .await?;
            }
            Sendable::File(file, caption) => {
                if let Some(caption) = caption {
                    $bot.send_photo($msg.chat.id, file)
                        .caption(caption)
                        .parse_mode(ParseMode::$format)
                        .await?;
                } else {
                    $bot.send_photo($msg.chat.id, file).await?;
                }
            }
        }
    };

    ($bot:expr, $msg:expr, $sendable:expr, format=$format:ident, spoiler=on) => {
        match $sendable {
            Sendable::Text(msg) => {
                $bot.send_message($msg.chat.id, msg)
                    .parse_mode(ParseMode::$format)
                    .await?;
            }
            Sendable::File(file, caption) => {
                if let Some(caption) = caption {
                    $bot.send_photo($msg.chat.id, file)
                        .caption(caption)
                        .parse_mode(ParseMode::$format)
                        .has_spoiler(true)
                        .await?;
                } else {
                    $bot.send_photo($msg.chat.id, file)
                        .has_spoiler(true)
                        .await?;
                }
            }
        }
    };
}

impl Sendable {
    pub fn builder() -> SendableBuilder<(), (), ()> {
        SendableBuilder {
            text: (),
            file: (),
            caption: (),
        }
    }

    pub fn text(s: impl Display) -> Self {
        Self::Text(s.to_string())
    }

    pub async fn send(self, bot: &Bot, msg: &Message) -> anyhow::Result<()> {
        let chat = msg.chat.id;
        match self {
            Sendable::Text(msg) => {
                bot.send_message(chat, msg).await?;
            }
            Sendable::File(file, caption) => {
                if let Some(caption) = caption {
                    bot.send_photo(chat, file).caption(caption).await?;
                } else {
                    bot.send_photo(chat, file).await?;
                }
            }
        }

        Ok(())
    }
}

pub struct SendableBuilder<T, F, C> {
    text: T,
    file: F,
    caption: C,
}

impl SendableBuilder<(), (), ()> {
    pub fn text(self, s: impl Display) -> SendableBuilder<String, (), ()> {
        SendableBuilder {
            text: s.to_string(),
            file: (),
            caption: (),
        }
    }

    pub fn url(self, u: impl IntoUrl) -> SendableBuilder<(), InputFile, ()> {
        SendableBuilder {
            file: InputFile::url(u.into_url().unwrap()),
            text: (),
            caption: (),
        }
    }
}

impl SendableBuilder<String, (), ()> {
    pub fn build(self) -> Sendable {
        Sendable::Text(self.text)
    }
}

impl SendableBuilder<(), InputFile, ()> {
    pub fn build(self) -> Sendable {
        Sendable::File(self.file, None)
    }

    pub fn caption(self, c: impl std::fmt::Display) -> SendableBuilder<(), InputFile, String> {
        SendableBuilder {
            text: (),
            file: self.file,
            caption: c.to_string(),
        }
    }
}

impl SendableBuilder<(), InputFile, String> {
    pub fn build(self) -> Sendable {
        Sendable::File(self.file, Some(self.caption))
    }
}
