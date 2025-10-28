use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;

use async_stream::try_stream;
use parking_lot::RwLock;
use tokio_stream::Stream;
use tonic::transport::Server;
use tonic::Response;

use crate::pb::messaging_service_server::{MessagingService, MessagingServiceServer};
use crate::pb::telemetry_command::Command;
use crate::pb::{
    self, Broker, Code, Endpoints, Message, MessageQueue, Resource, SendMessageResponse, Settings,
    Status, TelemetryCommand,
};
use crate::service::topic_config::TopicConfigManager;

pub struct GrpcMessagingServer {
    server_config: ServerConfig,
}

#[derive(Debug, Clone)]
pub struct ClientSettingManager {
    client_settings_map: Arc<RwLock<HashMap<String, Settings>>>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    store_path: String,
}

impl GrpcMessagingServer {
    pub fn new(server_config: ServerConfig) -> Self {
        Self { server_config }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        let service_inner =
            MessagingServiceServer::new(MessagingServer::new(self.server_config.clone())?);

        let addr = "0.0.0.0:8081".parse().unwrap();
        Server::builder()
            .add_service(service_inner)
            .serve(addr)
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct MessagingServer {
    setting_manager: ClientSettingManager,
    topic_config_manager: Arc<TopicConfigManager>,
}

impl MessagingServer {
    pub fn new(server_config: ServerConfig) -> Result<Self, anyhow::Error> {
        let topic_config_manager = TopicConfigManager::new(server_config.store_path.as_str())?;
        Ok(Self {
            setting_manager: ClientSettingManager::new(),
            topic_config_manager: Arc::new(topic_config_manager),
        })
    }

    pub fn get_message_queues(
        &self,
        topic: &str,
        endpoints: &Endpoints,
    ) -> Option<Vec<MessageQueue>> {
        let topic_config = self.topic_config_manager.get_topic_config(topic)?;
        let mut message_queues: Vec<MessageQueue> =
            Vec::with_capacity(topic_config.queue_num() as usize);
        let mut i = 0;

        while i < topic_config.queue_num() {
            let queue = MessageQueue {
                topic: Some(Resource {
                    name: topic.to_string(),
                    resource_namespace: "".to_string(),
                }),
                accept_message_types: vec![topic_config.topic_type() as i32],
                permission: 6,
                id: i as i32,
                broker: Some(Broker {
                    name: "broker".to_string(),
                    id: 0,
                    endpoints: Some(endpoints.clone()),
                }),
            };
            i += 1;
            message_queues.push(queue);
        }

        Some(message_queues)
    }

    pub async fn send_messages(&self, messages: &Vec<Message>) {}
}

#[tonic::async_trait]
impl MessagingService for MessagingServer {
    type TelemetryStream =
        Pin<Box<dyn Stream<Item = Result<pb::TelemetryCommand, tonic::Status>> + Send + 'static>>;
    type ReceiveMessageStream = tonic::Streaming<pb::ReceiveMessageResponse>;
    type PullMessageStream = tonic::Streaming<pb::PullMessageResponse>;
    async fn query_assignment(
        &self,
        _request: tonic::Request<pb::QueryAssignmentRequest>,
    ) -> Result<tonic::Response<pb::QueryAssignmentResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn query_route(
        &self,
        request: tonic::Request<pb::QueryRouteRequest>,
    ) -> Result<tonic::Response<pb::QueryRouteResponse>, tonic::Status> {
        let resource = request.get_ref().topic.clone();
        if resource.is_none() {
            return Err(tonic::Status::invalid_argument("no topic presented"));
        }
        let topic = resource.unwrap().name;
        let endpoints = request.get_ref().endpoints.clone().unwrap();
        if let Some(message_queues) = self.get_message_queues(&topic, &endpoints) {
            return Ok(tonic::Response::new(pb::QueryRouteResponse {
                status: Some(Status {
                    code: Code::Ok as i32,
                    message: "ok".to_string(),
                }),
                message_queues,
            }));
        } else {
            return Err(tonic::Status::not_found("topic route not found"));
        }
    }

    async fn heartbeat(
        &self,
        _request: tonic::Request<pb::HeartbeatRequest>,
    ) -> Result<tonic::Response<pb::HeartbeatResponse>, tonic::Status> {
        Ok(Response::new(pb::HeartbeatResponse {
            status: Some(pb::Status {
                code: Code::Ok as i32,
                message: "Ok".to_string(),
            }),
        }))
    }

    async fn send_message(
        &self,
        request: tonic::Request<pb::SendMessageRequest>,
    ) -> Result<tonic::Response<pb::SendMessageResponse>, tonic::Status> {
        let body = request.get_ref();
        self.send_messages(&body.messages).await;
        let response = SendMessageResponse {
            status: Some(pb::Status {
                code: Code::Ok as i32,
                message: Code::Ok.as_str_name().to_string(),
            }),
            entries: vec![],
        };
        Ok(tonic::Response::new(response))
    }

    async fn receive_message(
        &self,
        _request: tonic::Request<pb::ReceiveMessageRequest>,
    ) -> Result<tonic::Response<Self::ReceiveMessageStream>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn ack_message(
        &self,
        _request: tonic::Request<pb::AckMessageRequest>,
    ) -> Result<tonic::Response<pb::AckMessageResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn forward_message_to_dead_letter_queue(
        &self,
        _request: tonic::Request<pb::ForwardMessageToDeadLetterQueueRequest>,
    ) -> Result<tonic::Response<pb::ForwardMessageToDeadLetterQueueResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn pull_message(
        &self,
        _request: tonic::Request<pb::PullMessageRequest>,
    ) -> Result<tonic::Response<Self::PullMessageStream>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn update_offset(
        &self,
        _request: tonic::Request<pb::UpdateOffsetRequest>,
    ) -> Result<tonic::Response<pb::UpdateOffsetResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn get_offset(
        &self,
        _request: tonic::Request<pb::GetOffsetRequest>,
    ) -> Result<tonic::Response<pb::GetOffsetResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn query_offset(
        &self,
        _request: tonic::Request<pb::QueryOffsetRequest>,
    ) -> Result<tonic::Response<pb::QueryOffsetResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn end_transaction(
        &self,
        _request: tonic::Request<pb::EndTransactionRequest>,
    ) -> Result<tonic::Response<pb::EndTransactionResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn telemetry(
        &self,
        request: tonic::Request<tonic::Streaming<pb::TelemetryCommand>>,
    ) -> Result<tonic::Response<Self::TelemetryStream>, tonic::Status> {
        let mut stream = request.into_inner();
        let output = try_stream! {
            while let Ok(message) = stream.message().await {
                if let Some(command) = message {
                    if let Some(command) = command.command {
                        match command {
                            Command::Settings(settings) => {
                                //TODO: add detail implementation.
                                yield TelemetryCommand {
                                    status: Some(pb::Status {
                                        code: Code::Ok as i32,
                                        message: "ok".to_string(),
                                    }),
                                    command: Some(Command::Settings(settings.clone())),
                                }
                            }
                            _ => {

                            }
                        }
                    }
                }
            }
            println!("Still working on this command!")
        };
        Ok(Response::new(Box::pin(output)))
    }

    async fn notify_client_termination(
        &self,
        _request: tonic::Request<pb::NotifyClientTerminationRequest>,
    ) -> Result<tonic::Response<pb::NotifyClientTerminationResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn change_invisible_duration(
        &self,
        _request: tonic::Request<pb::ChangeInvisibleDurationRequest>,
    ) -> Result<tonic::Response<pb::ChangeInvisibleDurationResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
    }
}

impl ClientSettingManager {
    pub fn new() -> Self {
        let client_settings_map = Arc::new(RwLock::new(HashMap::new()));
        ClientSettingManager {
            client_settings_map,
        }
    }

    pub fn add_setting(&mut self, client_id: String, settings: Settings) {
        self.client_settings_map.write().insert(client_id, settings);
    }
}
