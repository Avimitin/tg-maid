use redis::Commands;
use std::{collections::HashSet, hash::Hash};

pub struct Cacher(r2d2::Pool<redis::Client>);

impl Cacher {
    pub fn new(client: redis::Client) -> Self {
        Self(
            r2d2::Pool::builder()
                .build(client)
                .expect("fail to construct a R2D2 Redis connection"),
        )
    }

    pub fn get_conn(&self) -> r2d2::PooledConnection<redis::Client> {
        self.0.get().expect("fail to get redis connection")
    }

    pub fn setup_subscribe_registry<'iter, Subscriber, Event, Relation>(
        &self,
        event_name: &str,
        iter: Relation,
    ) where
        Subscriber: Eq + Hash + std::fmt::Debug + redis::ToRedisArgs + 'iter,
        Event: Eq + Hash + std::fmt::Debug + std::fmt::Display + redis::ToRedisArgs + 'iter,
        Relation: Iterator<Item = (&'iter Subscriber, &'iter Vec<Event>)>,
    {
        iter.for_each(|(k, v)| {
            self.subscribe_event(event_name, k, v)
                .unwrap_or_else(|err| {
                    panic!(
                        "fail to initialize the {} subscribe registry \
                        when subscribe event {:?} for registrant {:?}: \
                        {err}",
                        event_name, v, k
                    )
                });
        });
    }

    pub fn event_pool<Event>(&self, event_name: &str) -> anyhow::Result<Vec<Event>>
    where
        Event: redis::FromRedisValue,
    {
        let event_pool_key = format!("REGISTRY_EVENT_POOL:{}", event_name);
        let events = self.get_conn().smembers(event_pool_key)?;
        Ok(events)
    }

    pub fn get_subscribers<Subscriber, Event>(
        &self,
        event_name: &str,
        event: &Event,
    ) -> anyhow::Result<Vec<Subscriber>>
    where
        Subscriber: redis::FromRedisValue,
        Event: redis::ToRedisArgs + std::fmt::Display,
    {
        let key = format!("SUBSCRIBE_REGISTRY:{}:{}", event_name, event);
        let subscriber = self.get_conn().smembers(key)?;
        Ok(subscriber)
    }

    // Create `event = [registrant]` key-value pair
    fn subscribe_event<Subscriber, Event>(
        &self,
        event_name: &str,
        registrant: &Subscriber,
        events: &Vec<Event>,
    ) -> anyhow::Result<()>
    where
        Subscriber: redis::ToRedisArgs,
        Event: redis::ToRedisArgs + std::fmt::Display,
    {
        let mut conn = self.get_conn();
        let event_pool_key = format!("REGISTRY_EVENT_POOL:{}", event_name);

        let search = format!("SUBSCRIBE_REGISTRY:{event_name}:*");
        let existing: HashSet<String> = conn.keys(&search)?;
        let mut popingin: HashSet<String> = HashSet::with_capacity(existing.len());

        for event in events {
            let key = format!("SUBSCRIBE_REGISTRY:{}:{}", event_name, event);
            let () = conn.sadd(key.as_str(), registrant)?;
            let () = conn.sadd(event_pool_key.as_str(), event)?;

            popingin.insert(key);
        }

        let garbage: Vec<String> = (&existing - &popingin).iter().cloned().collect();
        for event in garbage {
            let () = conn.srem(event, registrant)?;
        }

        Ok(())
    }
}

#[test]
fn test_event_registry() {
    dotenvy::dotenv().ok();
    let redis_addr = std::env::var("REDIS_ADDR").unwrap();
    let client = redis::Client::open(redis_addr).unwrap();
    let cacher = Cacher::new(client);

    let relation = std::collections::HashMap::from([
        ("foo", vec![1, 2, 3]),
        ("bar", vec![1, 2]),
        ("baz", vec![3, 4, 5]),
    ]);

    let name = "TestRegistry";
    cacher.setup_subscribe_registry(name, relation.iter());

    let mut events: Vec<i32> = cacher.event_pool(name).unwrap();
    events.sort();
    assert_eq!(events, [1, 2, 3, 4, 5]);

    let subscribers: Vec<String> = cacher.get_subscribers(name, &2_i32).unwrap();
    assert_eq!(subscribers.len(), 2);
    assert!(subscribers.iter().any(|x| x == "foo"));
    assert!(subscribers.iter().any(|x| x == "bar"));

    let subscribers: Vec<String> = cacher.get_subscribers(name, &3_i32).unwrap();
    assert_eq!(subscribers.len(), 2);
    assert!(subscribers.iter().any(|x| x == "foo"));
    assert!(subscribers.iter().any(|x| x == "baz"));

    // Now assuming "foo" unregister event `3`
    let relation = std::collections::HashMap::from([
        ("foo", vec![1, 2]),
        ("bar", vec![1, 2]),
        ("baz", vec![3, 4, 5]),
    ]);
    cacher.setup_subscribe_registry(name, relation.iter());
    let subscribers: Vec<String> = cacher.get_subscribers(name, &3_i32).unwrap();
    assert_eq!(subscribers.len(), 1);
    assert!(subscribers.iter().any(|x| x == "baz"));
}
