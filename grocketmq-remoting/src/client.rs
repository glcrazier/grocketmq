use tokio::net::{TcpSocket, TcpStream};

pub struct RemotingClient {
    stream: TcpStream,
}

impl RemotingClient {
    pub async fn new(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let addr = addr.parse()?;
        let socket = TcpSocket::new_v4()?;
        socket.set_nodelay(true)?;
        let stream = socket.connect(addr).await?;

        Ok(Self { stream })
    }
}
