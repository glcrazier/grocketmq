use std::sync::Arc;

use anyhow::{anyhow, Error};
use rocksdb::{WriteBatchWithTransaction, DB};
use serde::{Deserialize, Serialize};

use crate::store::db_get_usize;

#[derive(Debug, Clone)]
pub struct ConsumeQueueOffset {
    commitlog_offset: usize,
}
pub struct ConsumeQueue {
    topic: String,
    db: Arc<DB>,
    min_offset: usize,
    max_offset: usize,
    max_offset_key: String,
    min_offset_key: String,
}

const KEY_SEPARATOR: char = ';';
const FLAG_NORMAL: usize = 1;
const FLAG_MAX_OFFSET: usize = 2;
const FLAG_MIN_OFFSET: usize = 0;

/*
 * A consume queue represents mapping between offset under one topic and commitlog offset.
 * A consumer which subscribes a topic could poll messages by offset through the queue.
 * The key which represents the offset is composed of the following parts which are separated by special characters as ';':
 * 1. topic name
 * 2. offset type: 1 is normal, 2 is max offset, 0 is min offset.
 * 2. queue offset
 */
impl ConsumeQueue {
    pub fn new(topic: &str, db: Arc<DB>) -> Result<Self, anyhow::Error> {
        let max_offset_key = format!("{};{}", topic, FLAG_MAX_OFFSET);
        let min_offset_key = format!("{};{}", topic, FLAG_MIN_OFFSET);
        let max_offset = db_get_usize(&db, &max_offset_key, 0)?;
        let min_offset = db_get_usize(&db, &min_offset_key, 0)?;
        Ok(ConsumeQueue {
            topic: topic.to_string(),
            db,
            max_offset_key,
            min_offset_key,
            max_offset,
            min_offset,
        })
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
            self.max_offset
        } else {
            end_offset
        };
        Err(anyhow!("not implemented."))
    }

    pub fn add_offset(&mut self, pivot: usize, log: ConsumeQueueOffset) {
        let key = ConsumeQueue::build_offset_key(&self.topic, pivot);
        let value = log.encode();
        let mut batch = WriteBatchWithTransaction::new();
        batch.put(key, value);
        let result = self.db.write(batch);
        if result.is_ok() {
            self.max_offset += 1;
        }
    }

    fn build_offset_key(topic: &str, offset: usize) -> String {
        //TODO: offset should be encoded to bytes
        format!("{};{};{}", topic, FLAG_NORMAL, offset)
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
}
