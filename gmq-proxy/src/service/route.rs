use crate::remoting::client::{MQClient, TopicRouteData};

use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{mpsc, oneshot},
    time::{sleep, Instant},
};

pub struct RouteService {
    route_cache: Arc<RwLock<HashMap<String, RouteInfo>>>,
    client: Arc<MQClient>,
    route_refresh_tx: Option<mpsc::Sender<String>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    refresh_cache_timeout: Duration,
}

#[derive(Debug, Clone)]
enum RouteStatus {
    Ok,
    Inflight,
    Failure,
}

#[derive(Debug)]
struct RouteRequest {
    topic: String,
    tx: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone)]
struct RouteInfo {
    route: Option<TopicRouteData>,
    update_time: Instant,
    status: RouteStatus,
    refresh_timeout: Duration,
}

impl RouteInfo {
    pub fn new(refresh_timeout: Duration) -> Self {
        Self {
            route: None,
            update_time: Instant::now(),
            status: RouteStatus::Inflight,
            refresh_timeout,
        }
    }

    pub fn get_route(&self) -> Option<TopicRouteData> {
        return self.route.clone();
    }

    pub fn is_out_of_date(&self) -> bool {
        return self.update_time.elapsed() > self.refresh_timeout;
    }

    pub fn get_status(&self) -> RouteStatus {
        self.status.clone()
    }

    pub fn set_route_data(&mut self, data: TopicRouteData) {
        self.route = Some(data);
        self.update_time = Instant::now();
        self.status = RouteStatus::Ok;
    }

    pub fn set_status(&mut self, status: RouteStatus) {
        self.status = status;
    }
}

impl RouteService {
    pub fn new(client: &Arc<MQClient>, refresh_cache_timeout: Duration) -> Self {
        Self {
            route_cache: Arc::new(RwLock::new(HashMap::with_capacity(1024))),
            client: Arc::clone(client),
            shutdown_tx: None,
            refresh_cache_timeout,
            route_refresh_tx: None,
        }
    }

    pub async fn start(&mut self) {
        let (route_refresh_tx, mut route_refresh_rx) = mpsc::channel(1024);
        self.route_refresh_tx = Some(route_refresh_tx);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);
        let route_cache = self.route_cache.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    Some(topic) = route_refresh_rx.recv() => {
                        let query_result = client.query_route(&topic).await;
                        if let Ok(route_data) = query_result {
                            route_cache.write().entry(topic.clone()).and_modify(|r| r.set_route_data(route_data));
                        } else {
                            println!("query route {:?} faiured.", topic);
                        }
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
        });
    }

    async fn trigger_refresh_route(&self, topic: &str) {
        if let Some(tx) = self.route_refresh_tx.as_ref() {
            let _ = tx.send(topic.to_string()).await;
        }
    }

    pub async fn get_topic_route(&self, topic: &str) -> Option<TopicRouteData> {
        if let Some(route_info) = self.route_cache.read().get(topic).map(|v| v.clone()) {
            if let RouteStatus::Inflight = route_info.get_status() {
                let max_block_time = self.refresh_cache_timeout;
                let mut block_time = Duration::from_millis(100);
                while block_time < max_block_time {
                    sleep(block_time).await;
                    if let Some(route_info) = self.route_cache.read().get(topic).map(|v| v.clone())
                    {
                        if let RouteStatus::Ok = route_info.get_status() {
                            return route_info.get_route();
                        }
                    }
                    block_time = block_time.saturating_mul(2);
                }
                return None;
            }
            if let RouteStatus::Ok = route_info.get_status() {
                if route_info.is_out_of_date() {
                    self.trigger_refresh_route(topic).await;
                }
                return route_info.get_route();
            }
        }

        self.route_cache
            .write()
            .entry(topic.to_string())
            .or_insert(RouteInfo::new(self.refresh_cache_timeout));
        let route_info_option = self.route_cache.read().get(topic).map(|v| v.clone());
        if let Some(route_info) = route_info_option {
            match route_info.get_status() {
                RouteStatus::Ok => {
                    return route_info.get_route();
                }
                _ => {
                    let route_cache = Arc::clone(&self.route_cache);
                    let client = self.client.clone();
                    let topic2 = topic.to_string();
                    let _ = tokio::spawn(async move {
                        let query_result = client.query_route(topic2.as_str()).await;
                        match query_result {
                            Ok(route_data) => {
                                let mut cache = route_cache.write();
                                let route_info = cache.get_mut(topic2.as_str());
                                match route_info {
                                    None => {
                                        println!(
                                            "topic route {:?} doesn't exist in cache, drop.",
                                            topic2
                                        );
                                    }
                                    Some(route_info_mut) => {
                                        route_info_mut.set_route_data(route_data);
                                    }
                                }
                            }
                            Err(e) => {
                                route_cache
                                    .write()
                                    .entry(topic2.clone())
                                    .and_modify(|r| r.set_status(RouteStatus::Failure));
                                print!("query topic route = {:?} failed, error = {:?}.", topic2, e);
                            }
                        }
                    })
                    .await;
                    return self
                        .route_cache
                        .read()
                        .get(topic)
                        .map(|v| v.get_route())
                        .flatten();
                }
            }
        } else {
            return None;
        }
    }
}
