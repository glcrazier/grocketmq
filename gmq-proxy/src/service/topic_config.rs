use anyhow::Error;
use log::error;
use rocksdb::DB;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum TopicType {
    NORMAL,
    DELAY,
    FIFO,
    TRANSACTION,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicConfig {
    name: String,
    queue_num: u32,
    topic_type: TopicType,
}

impl TopicConfig {
    pub fn new(name: String, queue_num: u32, topic_type: TopicType) -> Self {
        Self {
            name,
            queue_num,
            topic_type,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn queue_num(&self) -> u32 {
        self.queue_num
    }

    pub fn topic_type(&self) -> TopicType {
        self.topic_type.clone()
    }
}

#[derive(Debug)]
pub struct TopicConfigManager {
    db: Arc<DB>,
}

impl TopicConfigManager {
    pub fn new(path: &str) -> Result<Self, Error> {
        let topic_config_path = path.to_string() + "/topic_config";
        let result = DB::open_default(topic_config_path);
        match result {
            Ok(db) => return Ok(Self { db: Arc::new(db) }),
            Err(e) => return Err(anyhow::anyhow!(e)),
        }
    }

    pub fn get_topic_config(&self, topic_name: &str) -> Option<TopicConfig> {
        let result = self.db.get(topic_name.as_bytes());
        if let Ok(data) = result {
            if let Some(u) = data {
                match serde_json::from_slice(&u) {
                    Ok(config) => return config,
                    Err(e) => {
                        error!(
                            "deserialize topic config {} from db error = {}",
                            topic_name, e
                        );
                    }
                }
            }
        } else {
            error!(
                "get topic {} from db error = {}",
                topic_name,
                result.unwrap_err()
            );
        }
        None
    }

    pub fn add_or_update_topic(&self, config: TopicConfig) -> Result<(), anyhow::Error> {
        let topic_name = config.name().to_string();
        if topic_name.is_empty() {
            return Err(anyhow::Error::msg("topic name is empty"));
        }
        let value = serde_json::to_vec(&config).unwrap();
        if let Err(e) = self.db.put(topic_name.as_bytes(), &value) {
            return Err(anyhow::Error::new(e));
        }
        Ok(())
    }

    pub fn delete_topic(&self, topic_name: &str) -> Result<(), anyhow::Error> {
        if let Err(e) = self.db.delete(topic_name.as_bytes()) {
            return Err(anyhow::Error::new(e));
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static MTX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_add_or_update_config() {
        let _m = MTX.lock();
        let topic_config_manager = TopicConfigManager::new("../target").unwrap();
        let topic_config = TopicConfig {
            name: "test1".to_string(),
            queue_num: 3,
            topic_type: TopicType::NORMAL,
        };
        topic_config_manager
            .add_or_update_topic(topic_config)
            .unwrap();
        let result = topic_config_manager.get_topic_config("test1").unwrap();
        assert_eq!("test1", result.name());
    }

    #[test]
    fn test_delete_topic() {
        let _m = MTX.lock();
        let topic_config_manager = TopicConfigManager::new("../target").unwrap();
        let topic = TopicConfig {
            name: "test1".to_string(),
            queue_num: 3,
            topic_type: TopicType::NORMAL,
        };
        topic_config_manager.add_or_update_topic(topic).unwrap();
        topic_config_manager.delete_topic("test1").unwrap();
        assert!(topic_config_manager.get_topic_config("test1").is_none());
    }
}
