use crate::app::AppData;
use redis::Commands;

pub fn hit(data: AppData) -> anyhow::Result<u32> {
    Ok(data.cacher.get_conn().incr("KSYX_HIT_COUNTER", 1)?)
}
