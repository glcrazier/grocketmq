use std::{collections::HashMap, fs, path::Path};

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
    topic_type: TopicType,
}

impl TopicConfig {
    pub fn new(name: String, topic_type: TopicType) -> Self {
        Self { name, topic_type }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn topic_type(&self) -> &TopicType {
        &self.topic_type
    }
}

#[derive(Debug)]
pub struct TopicConfigManager {
    path: String,
    topic_config_table: HashMap<String, TopicConfig>,
    backup_path: String,
}

impl TopicConfigManager {
    pub fn new(path: &str) -> Self {
        let topic_config_path = path.to_string() + "/topic_config.json";
        let backup_path = topic_config_path.clone() + ".bak";
        Self {
            topic_config_table: HashMap::new(),
            path: topic_config_path,
            backup_path,
        }
    }

    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(self.path.as_str());
        let result = fs::read_to_string(path);
        if let Ok(data) = result {
            let topic_config_table: HashMap<String, TopicConfig> = serde_json::from_str(&data)?;
            self.topic_config_table = topic_config_table;
        } else {
            fs::write(path, "{}")?;
        }
        Ok(())
    }

    pub fn get_topic_config(&self, topic_name: &str) -> Option<&TopicConfig> {
        self.topic_config_table.get(topic_name)
    }

    pub fn add_or_update_topic(
        &mut self,
        config: TopicConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let topic_name = config.name().to_string();
        if topic_name.is_empty() {
            return Err("topic name is empty".into());
        }
        self.topic_config_table.insert(topic_name, config);
        self.persist()
    }

    pub fn delete_topic(&mut self, topic_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.topic_config_table.remove(topic_name);
        if result.is_some() {
            return self.persist();
        }
        Ok(())
    }

    fn persist(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::copy(self.path.as_str(), self.backup_path.as_str())?;
        let data = serde_json::to_string(&self.topic_config_table)?;
        fs::write(self.path.as_str(), data)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static MTX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    #[test]
    fn test_load_config() {
        let _m = MTX.lock();
        let data = json!({
            "test1": {
                "name": "test1",
                "topic_type": "NORMAL",
            }
        });
        fs::write("./topic_config.json", data.to_string()).unwrap();
        let mut topic_config_manager = TopicConfigManager::new("./");
        topic_config_manager.load().unwrap();
        let topic_config = topic_config_manager.get_topic_config("test1");
        assert!(topic_config.is_some());
        assert_eq!(topic_config.unwrap().name, "test1");
        assert!(matches!(
            topic_config.unwrap().topic_type,
            TopicType::NORMAL
        ));
        fs::remove_file("./topic_config.json").unwrap();
    }

    #[test]
    fn test_add_or_update_config() {
        let _m = MTX.lock();
        let mut topic_config_manager = TopicConfigManager::new("./");
        topic_config_manager.load().unwrap();
        let topic_config = TopicConfig {
            name: "test1".to_string(),
            topic_type: TopicType::NORMAL,
        };
        topic_config_manager
            .add_or_update_topic(topic_config)
            .unwrap();
        fs::remove_file("./topic_config.json").unwrap();
        fs::remove_file("./topic_config.json.bak").unwrap();
    }

    #[test]
    fn test_delete_topic() {
        let _m = MTX.lock();
        let mut topic_config_manager = TopicConfigManager::new("./");
        topic_config_manager.load().unwrap();
        let topic = TopicConfig {
            name: "test1".to_string(),
            topic_type: TopicType::NORMAL,
        };
        topic_config_manager
            .add_or_update_topic(topic)
            .unwrap();
        topic_config_manager.delete_topic("test1").unwrap();
        assert!(topic_config_manager.get_topic_config("test1").is_none());
        fs::remove_file("./topic_config.json").unwrap();
        fs::remove_file("./topic_config.json.bak").unwrap();
    }
}
