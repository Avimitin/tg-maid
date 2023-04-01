use std::{
    collections::HashMap,
    fmt::Display,
    future::Future,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use tokio::sync::watch;
use typed_builder::TypedBuilder;

use crate::app::AppData;

#[derive(Debug, Default, Clone, Copy)]
pub struct State<S>(pub S);

impl<S> Deref for State<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(TypedBuilder)]
pub struct EventWatcher<S> {
    pub bot: teloxide::Bot,
    pub data: AppData,
    #[builder(setter( transform = |s: S| Arc::new(State(s)) ))]
    pub state: Arc<State<S>>,
}

impl<S> Clone for EventWatcher<S> {
    fn clone(&self) -> Self {
        // bot & data is already wrapped by Arc
        Self {
            bot: self.bot.clone(),
            data: self.data.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

pub trait Promise: Future<Output = anyhow::Result<()>> + Send + 'static {}
impl<T> Promise for T where T: Future<Output = anyhow::Result<()>> + Send + 'static {}

impl<S> EventWatcher<S> {
    pub fn start<P, T>(self, name: impl Display, interval_secs: u64, task: T)
    where
        P: Promise,
        S: Send + Sync + 'static,
        T: Fn(EventWatcher<S>) -> P + Sync + Send + 'static,
    {
        let (tx, rx) = watch::channel(1_u8);
        let mut heartbeat = tokio::time::interval(Duration::from_secs(interval_secs));

        tokio::spawn(async move {
            loop {
                let watcher = self.clone();
                let mut rx = rx.clone();

                tokio::select! {
                    _ = rx.changed() => {
                        break;
                    }
                    _ = heartbeat.tick() => {
                        if let Err(err) = task(watcher).await {
                            tracing::error!("{}", err)
                        }
                    }
                }
            }
        });

        let name = name.to_string();

        let quit_on_ctrl_c = || async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Quiting event watcher for {}...", name);
            tx.send(0)
                .unwrap_or_else(|_| panic!("fail to send signal into event watcher {}", name));
        };

        tokio::spawn(quit_on_ctrl_c());
    }
}

pub struct Registry<Registrant, Event> {
    relation: HashMap<Rc<Registrant>, Vec<Rc<Event>>>,
    event_pool: Vec<Rc<Event>>,
    cache: HashMap<Rc<Event>, Vec<Rc<Registrant>>>,
}

impl<R: Hash + Clone + PartialEq + Eq, E: Ord + Clone + Hash> Registry<R, E> {
    pub fn new(relation: HashMap<R, Vec<E>>) -> Self {
        let relation = relation
            .into_iter()
            .map(|(registrant, events)| {
                (
                    Rc::new(registrant),
                    events.into_iter().map(|e| Rc::new(e)).collect::<Vec<_>>(),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut event_pool = relation
            .values()
            .flatten()
            .cloned() // Perform Rc::clone
            .collect::<Vec<_>>();
        event_pool.sort();
        event_pool.dedup();

        Self {
            relation,
            event_pool,
            cache: HashMap::new(),
        }
    }

    pub fn register(&mut self, registrant: R, events: &[E]) {
        let events = events
            .iter()
            .cloned()
            .map(|e| {
                self.cache.remove(&e);
                Rc::new(e)
            })
            .collect::<Vec<_>>();

        self.relation
            .entry(Rc::new(registrant))
            .and_modify(|exist| exist.extend_from_slice(events.as_slice()))
            .or_insert(events);
    }

    #[inline]
    pub fn get_registrant_from_cache(&self, event: &E) -> Option<Vec<&R>> {
        self.cache
            .get(event)
            .map(|registrants| registrants.iter().map(|inner| inner.as_ref()).collect())
    }

    pub fn find_registrants_by_event(&mut self, event: &E) -> Vec<&R> {
        if self.cache.contains_key(event) {
            return self.get_registrant_from_cache(event).unwrap();
        }

        let mut matched_event = None;

        let registrants: Vec<_> = self
            .relation
            .iter()
            .filter_map(|(registrant, events)| {
                let matched = events
                    .iter()
                    .find(|subscribed| subscribed.as_ref() == event);
                if let Some(matched) = matched {
                    matched_event.replace(matched);
                    Some(Rc::clone(registrant))
                } else {
                    None
                }
            })
            .collect();

        if registrants.is_empty() || matched_event.is_none() {
            return Vec::new();
        }

        let matched_event = Rc::clone(matched_event.unwrap());
        self.cache.insert(matched_event, registrants);
        self.get_registrant_from_cache(event).unwrap()
    }

    pub fn pool(&self) -> Vec<&E> {
        self.event_pool.iter().map(|inner| inner.as_ref()).collect()
    }
}
