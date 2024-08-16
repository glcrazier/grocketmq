use crate::pb;

use super::topic_config::TopicConfigManager;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct RouteService {
    topic_config_manager: Arc<RwLock<TopicConfigManager>>,
}

pub struct Route {}

impl RouteService {
    pub fn get_topic_route(&self, topic_name: &str) {}
}
