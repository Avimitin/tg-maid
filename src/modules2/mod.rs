// Provider Module
pub mod archlinux;
pub mod currency;
pub mod nsfw;

// Data Module
pub mod cache;
pub mod http;

// Every module should provide a function that turn user input to [`Sendable`]
use reqwest::IntoUrl;
use std::fmt::Display;
use teloxide::types::InputFile;

pub enum Sendable {
    Text(String),
    File(InputFile, Option<String>),
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
