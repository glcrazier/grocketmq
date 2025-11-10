use anyhow::anyhow;
use rocksdb::DB;
use std::sync::Arc;

pub mod consume_queue;
pub mod store;

fn db_get_u64(db: &Arc<DB>, key: impl AsRef<[u8]>, default_value: u64) -> Result<u64, anyhow::Error> {
    let data = db.get(key)?;
    if let Some(data) = data {
        if data.len() != 8 {
            return Err(anyhow!("The value size must be 8"));
        } else {
            let d: [u8; 8] = data.as_slice().try_into()?;
            return Ok(u64::from_be_bytes(d));
        }
    } else {
        return Ok(default_value);
    }
}
