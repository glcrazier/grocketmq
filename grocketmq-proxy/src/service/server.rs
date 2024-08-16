use std::error::Error;

use tonic::transport::Server;

use crate::pb;
use crate::pb::messaging_service_server::{MessagingService, MessagingServiceServer};

pub struct GrpcMessagingServer {}

impl GrpcMessagingServer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let service_inner = MessagingServiceServer::new(MessagingServer::default());

        let addr = "0.0.0.0:8081".parse().unwrap();
        Server::builder()
            .add_service(service_inner)
            .serve(addr)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MessagingServer {}

#[tonic::async_trait]
impl MessagingService for MessagingServer {
    type TelemetryStream = tonic::Streaming<pb::TelemetryCommand>;
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
        Err(tonic::Status::aborted("not implemented"))
    }

    async fn send_message(
        &self,
        _request: tonic::Request<pb::SendMessageRequest>,
    ) -> Result<tonic::Response<pb::SendMessageResponse>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
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
        _request: tonic::Request<tonic::Streaming<pb::TelemetryCommand>>,
    ) -> Result<tonic::Response<Self::TelemetryStream>, tonic::Status> {
        Err(tonic::Status::aborted("not implemented"))
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
