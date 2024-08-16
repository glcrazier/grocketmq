use grocketmq_proxy::service::server::GrpcMessagingServer;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let mut server = GrpcMessagingServer::new();
    let result = server.start().await;
    println!("Result: {:?}", result);
    
}