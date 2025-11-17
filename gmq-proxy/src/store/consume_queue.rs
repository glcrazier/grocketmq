use std::sync::Arc;

use anyhow::anyhow;
use rocksdb::{Direction, IteratorMode, WriteBatchWithTransaction, DB};

use crate::store::db_get_u64;

#[derive(Debug, Clone)]
pub struct ConsumeQueueOffset {
    file_index: u64,
    in_file_offset: u64,
}
pub struct ConsumeQueue {
    db: Arc<DB>,
    min_offset: u64,
    max_offset: u64,
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
 * The key which represents the offset is composed of the following parts:
 * 1. offset type, 1 byte, 0 represents normal, 1 represents min offset, 2 represents max offset.
 * 2. topic id, which represents the uniqueness of this topic, it requires 8 bytes;
 * 3. queue offset, it requires 8 bytes;
 */
impl ConsumeQueue {
    pub fn new(topic_id: u64, db: Arc<DB>) -> Result<Self, anyhow::Error> {
        let max_offset_key = ConsumeQueue::build_max_offset_key(topic_id);
        let min_offset_key = ConsumeQueue::build_min_offset_key(topic_id);
        let max_offset = db_get_u64(&db, &max_offset_key, 0)?;
        let min_offset = db_get_u64(&db, &min_offset_key, 0)?;
        let offset_prefix = ConsumeQueue::build_prefix_key(topic_id);
        Ok(ConsumeQueue {
            db,
            max_offset_key,
            min_offset_key,
            max_offset,
            min_offset,
            offset_prefix,
        })
    }

    fn build_max_offset_key(topic_code: u64) -> Vec<u8> {
        let mut result = Vec::with_capacity(9);
        result.push(FLAG_MAX_OFFSET);
        result.append(&mut topic_code.to_be_bytes().to_vec());
        result
    }

    fn build_min_offset_key(topic_code: u64) -> Vec<u8> {
        let mut result = Vec::with_capacity(9);
        result.push(FLAG_MIN_OFFSET);
        result.append(&mut topic_code.to_be_bytes().to_vec());
        result
    }

    fn build_prefix_key(topic_id: u64) -> Vec<u8> {
        let mut result = Vec::new();
        result.push(FLAG_NORMAL);
        result.append(&mut topic_id.to_be_bytes().to_vec());
        result

    }

    pub fn query_offset_list(
        &self,
        start_offset: u64,
        end_offset: u64,
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
        let mut result = Vec::with_capacity(count as usize);
        let mut current = 0;
        while current < count {
            if let Some(Ok(data)) = iter.next() {
                if let Ok(value) = ConsumeQueueOffset::decode(data.1.to_vec()) {
                        result.push(value);
                    }
            }
            current += 1;
        }

        Ok(result)
    }

    pub fn add_offset(
        &mut self,
        offset: u64,
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

    fn build_offset_key(&self, offset: u64) -> Vec<u8> {
        let mut result = Vec::new();
        result.append(&mut self.offset_prefix.clone());
        result.append(&mut offset.to_be_bytes().to_vec());
        result
    }
}

impl ConsumeQueueOffset {
    pub fn decode(data: Vec<u8>) -> Result<Self, anyhow::Error> {
        if data.len() != 16 {
            return Err(anyhow!("invalid data length"));
        }
        let file_index = u64::from_be_bytes(data[0..8].try_into().unwrap());
        let in_file_offset = u64::from_be_bytes(data[8..16].try_into().unwrap());
        Ok(Self {
            file_index,
            in_file_offset
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(16);
        result.append(&mut self.file_index.to_be_bytes().to_vec());
        result.append(&mut self.in_file_offset.to_be_bytes().to_vec());
        result
    }

    pub fn new(file_index: u64, in_file_offset: u64) -> Self {
        Self { file_index, in_file_offset }
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
        consume_queue.add_offset(1, ConsumeQueueOffset::new(1, 1))?;
        consume_queue.add_offset(2, ConsumeQueueOffset::new(1, 2))?;

        let offset_list = consume_queue.query_offset_list(1, 4)?;
        assert_eq!(2, offset_list.len());
        let offset = offset_list.get(0);
        println!("offset {:?}", offset.unwrap());
        Ok(())
    }
}
