use std::sync::Arc;

use anyhow::anyhow;
use rocksdb::{Direction, IteratorMode, WriteBatchWithTransaction, DB};

use crate::store::db_get_usize;

#[derive(Debug, Clone)]
pub struct ConsumeQueueOffset {
    commitlog_offset: usize,
}
pub struct ConsumeQueue {
    db: Arc<DB>,
    min_offset: usize,
    max_offset: usize,
    max_offset_key: Vec<u8>,
    min_offset_key: Vec<u8>,
    offset_prefix: Vec<u8>,
}

const FLAG_NORMAL: u8 = 1;
const FLAG_MAX_OFFSET: u8 = 2;
const FLAG_MIN_OFFSET: u8 = 0;

/*
 * A consume queue represents mapping between offset under one topic and commitlog offset.
 * A consumer which subscribes a topic could poll messages by offset through the queue.
 * The key which represents the offset is composed of the following parts which are separated by special characters as ';':
 * 1. topic name
 * 2. offset type: 1 is normal, 2 is max offset, 0 is min offset.
 * 2. queue offset
 */
impl ConsumeQueue {
    pub fn new(topic_code: usize, db: Arc<DB>) -> Result<Self, anyhow::Error> {
        let max_offset_key = ConsumeQueue::build_prefix_key(topic_code, FLAG_MAX_OFFSET );
        let min_offset_key = ConsumeQueue::build_prefix_key(topic_code, FLAG_MIN_OFFSET);
        let max_offset = db_get_usize(&db, &max_offset_key, 0)?;
        let min_offset = db_get_usize(&db, &min_offset_key, 0)?;
        let offset_prefix = ConsumeQueue::build_prefix_key(topic_code, FLAG_NORMAL);
        Ok(ConsumeQueue {
            db,
            max_offset_key,
            min_offset_key,
            max_offset,
            min_offset,
            offset_prefix,
        })
    }

    fn build_prefix_key(topic_code: usize, key_type: u8) -> Vec<u8> {
        let mut result = Vec::new();
        result.append(&mut topic_code.to_be_bytes().to_vec());
        result.push(key_type);
        result

    }

    pub fn query_offset_list(
        &self,
        start_offset: usize,
        end_offset: usize,
    ) -> Result<Vec<ConsumeQueueOffset>, anyhow::Error> {
        let fixed_start_offset = if start_offset < self.min_offset {
            self.min_offset
        } else {
            start_offset
        };
        let fixed_end_offset = if end_offset > self.max_offset {
            self.max_offset + 1
        } else {
            end_offset
        };
        let count = fixed_end_offset - fixed_start_offset;
        if count <= 0 {
            return Err(anyhow!("invalid range to search"));
        }
        let start_key = self.build_offset_key(fixed_start_offset);
        let mut iter = self
            .db
            .iterator(IteratorMode::From(&start_key, Direction::Forward));
        let mut result = Vec::with_capacity(count);
        let mut current = 0;
        while current < count {
            if let Some(Ok(data)) = iter.next() {
                if let Ok(value) = String::from_utf8(data.1.to_vec()) {
                    if let Ok(value) = ConsumeQueueOffset::decode(&value) {
                        result.push(value);
                    }
                }
            }
            current += 1;
        }

        Ok(result)
    }

    pub fn add_offset(
        &mut self,
        offset: usize,
        log: ConsumeQueueOffset,
    ) -> Result<(), anyhow::Error> {
        let key = self.build_offset_key(offset);
        let value = log.encode();
        let mut batch = WriteBatchWithTransaction::new();
        batch.put(key, value);
        batch.put(self.max_offset_key.clone(), offset.to_be_bytes());
        if let Err(e) = self.db.write(batch) {
            return Err(anyhow::Error::new(e));
        } else {
            self.max_offset = offset;
            return Ok(());
        }
    }

    fn build_offset_key(&self, offset: usize) -> Vec<u8> {
        let mut result = Vec::new();
        result.append(&mut self.offset_prefix.clone());
        result.append(&mut offset.to_be_bytes().to_vec());
        result
    }
}

impl ConsumeQueueOffset {
    pub fn decode(raw_data: &str) -> Result<Self, anyhow::Error> {
        let commitlog_offset = raw_data
            .parse::<usize>()
            .map_err(|e| anyhow!("Failed to parse commitlog offset '{}': {}", raw_data, e))?;

        Ok(ConsumeQueueOffset { commitlog_offset })
    }

    pub fn encode(&self) -> String {
        self.commitlog_offset.to_string()
    }

    pub fn new(commitlog_offset: usize) -> Self {
        Self { commitlog_offset }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_consume_queue() -> Result<(), anyhow::Error> {
        let mut temp_dir = env::temp_dir();
        temp_dir.push("test_rocksdb_consume_queue");
        let db = DB::open_default(temp_dir.as_path())?;
        let db = Arc::new(db);

        let mut consume_queue = ConsumeQueue::new(0, db)?;
        consume_queue.add_offset(1, ConsumeQueueOffset::new(1))?;
        consume_queue.add_offset(2, ConsumeQueueOffset::new(2))?;

        let offset_list = consume_queue.query_offset_list(1, 4)?;
        assert_eq!(2, offset_list.len());
        let offset = offset_list.get(0);
        println!("offset {:?}", offset.unwrap());
        Ok(())
    }
}
