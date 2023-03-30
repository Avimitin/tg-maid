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
}
