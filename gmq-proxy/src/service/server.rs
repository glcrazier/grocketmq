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
use crate::pb::{self, Code, Message, SendMessageResponse, Settings, TelemetryCommand};

pub struct GrpcMessagingServer {}

#[derive(Debug, Clone)]
pub struct ClientSettingManager {
    client_settings_map: Arc<RwLock<HashMap<String, Settings>>>,
}

impl GrpcMessagingServer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let service_inner = MessagingServiceServer::new(MessagingServer::new());

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
}

impl MessagingServer {
    pub fn new() -> Self {
        Self {
            setting_manager: ClientSettingManager::new(),
        }
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
        _request: tonic::Request<pb::QueryRouteRequest>,
    ) -> Result<tonic::Response<pb::QueryRouteResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
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
