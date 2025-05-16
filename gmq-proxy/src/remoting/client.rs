use std::collections::HashMap;

use anyhow::anyhow;
use gmq_remoting::{channel::Channel, common::command::Command};
use serde::Deserialize;

use crate::common::{Error, RequestCode};

#[derive(Debug)]
pub struct MQClient {
    addr: String,
    channel: Option<Channel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueData {
    #[serde(alias = "brokerName")]
    broker_name: String,
    #[serde(alias = "readQueueNums")]
    read_queue_nums: i32,
    #[serde(alias = "writeQueueNums")]
    write_queue_nums: i32,
    perm: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrokerData {
    cluster: String,
    #[serde(alias = "brokerName")]
    broker_name: String,
    #[serde(alias = "brokerAddrs")]
    broker_addrs: HashMap<i64, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TopicRouteData {
    #[serde(alias = "queueDatas")]
    queue_datas: Vec<QueueData>,
    #[serde(alias = "brokerDatas")]
    broker_datas: Vec<BrokerData>,
}

impl MQClient {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
            channel: None,
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        let result = Channel::new(&self.addr).await;
        match result {
            Ok(channel) => {
                self.channel = Some(channel);
                return Ok(());
            }
            Err(e) => {
                return Err(Error::InternalError(e.into()));
            }
        }
    }

    pub async fn query_route(&self, topic: &str) -> Result<TopicRouteData, Error> {
        if let Some(channel) = self.channel.as_ref() {
            let mut headers = HashMap::new();
            headers.insert("topic".to_string(), topic.to_string());
            headers.insert("acceptStandardJsonOnly".to_string(), "true".to_string());
            let cmd = Command::new_with_header(RequestCode::GetTopicRouteInfo as u8, headers);
            let result = channel.request(cmd).await;
            match result {
                Ok(command) => {
                    if let Some(body) = command.body() {
                        return serde_json::from_slice(body).map_err(|e| {
                            return Error::TopicNotFound(topic.to_string(), e.into());
                        });
                    } else {
                        return Err(Error::TopicNotFound(
                            topic.to_string(),
                            anyhow!("no body in response"),
                        ));
                    }
                }
                Err(e) => {
                    return Err(Error::TopicNotFound(topic.to_string(), e.into()));
                }
            }
        } else {
            Err(Error::InternalError(anyhow!("channel is not ready")))
        }
    }

    pub async fn shutdown(self) {
        if let Some(channel) = self.channel.as_ref() {
            channel.shutdown().await;
        }
    }
}
