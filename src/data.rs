use reqwest::IntoUrl;
use std::fmt::Display;
use std::ops::Deref;
use std::sync::Arc;
use teloxide::types::InputFile;

use crate::cache::Cacher;
use crate::provider::HttpClient;

pub struct AppData(Arc<RuntimeData>);

impl From<RuntimeData> for AppData {
    fn from(data: RuntimeData) -> Self {
        Self(Arc::new(data))
    }
}

impl Clone for AppData {
    fn clone(&self) -> Self {
        AppData(Arc::clone(&self.0))
    }
}

impl Deref for AppData {
    type Target = Arc<RuntimeData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(typed_builder::TypedBuilder)]
pub struct RuntimeData {
    pub cacher: Cacher,
    pub requester: HttpClient,
}

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
