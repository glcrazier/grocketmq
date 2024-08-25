use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    net::SocketAddr,
    time::Duration,
};

use tokio::{
    net::{TcpSocket, TcpStream},
    select,
    sync::{mpsc, oneshot},
    time::timeout,
};

use crate::common::command::Command;

#[derive(Debug)]
pub struct Channel {
    command_sender: mpsc::Sender<Request>,
    timeout: Duration,
}

type StdError = Box<dyn std::error::Error + Send + 'static>;

struct Request {
    commmand: Command,
    write_tx: oneshot::Sender<Result<(), StdError>>,
    response_tx: oneshot::Sender<Command>,
}

/**
 * A channel sends and receives Command messages.
 */
impl Channel {
    pub async fn new(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let addr = addr.parse()?;
        let (tx, mut rx) = mpsc::channel(1024);
        let command_sender: mpsc::Sender<Request> = tx;
        let mut response_table: HashMap<usize, oneshot::Sender<Command>> = HashMap::new();
        tokio::spawn(async move {
            let mut stream: Option<TcpStream> = None;
            loop {
                select! {
                    Some(request) = rx.recv() => {
                        if stream.is_none() {
                            stream = Channel::new_stream(addr).await.ok();
                        }
                        if let Some(stream) = stream.as_mut() {
                            let opaque = request.commmand.opaque();
                            let result = Channel::write(stream, request.commmand).await;
                            let write_tx = request.write_tx;
                            if write_tx.send(result).is_ok() {
                                response_table.insert(opaque, request.response_tx);
                            }
                        } else {
                            let _ = request.write_tx.send(Err(Box::new(Error::new(ErrorKind::AddrNotAvailable, "no stream available"))));
                        }
                    }
                    Some(_) = Channel::stream_readable(&stream) => {
                    }
                }
            }
        });
        Ok(Self {
            command_sender,
            timeout: Duration::from_secs(10),
        })
    }

    async fn stream_readable(stream: &Option<TcpStream>) -> Option<()> {
        if let Some(stream) = stream {
            stream.readable().await.ok()
        } else {
            None
        }
    }

    async fn new_stream(addr: SocketAddr) -> Result<TcpStream, Box<dyn std::error::Error>> {
        let socket = TcpSocket::new_v4()?;
        socket.set_nodelay(true)?;
        let stream = socket.connect(addr).await?;
        Ok(stream)
    }

    async fn write(stream: &mut TcpStream, cmd: Command) -> Result<(), StdError> {
        let encoded_data = cmd.encode();
        let len = encoded_data.len();
        let mut written_bytes = 0;
        loop {
            if let Err(e) = stream.writable().await {
                return Err(Box::new(e));
            }
            let raw_bytes = &encoded_data[written_bytes..len];
            match stream.try_write(raw_bytes) {
                Ok(n) => {
                    written_bytes += n;
                    if written_bytes == len {
                        break;
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        continue;
                    }
                    return Err(Box::new(e));
                }
            }
        }
        Ok(())
    }

    pub async fn request(&self, cmd: Command) -> Result<Command, Box<dyn std::error::Error>> {
        let (write_tx, write_rx) = oneshot::channel();
        let (response_tx, response_rx) = oneshot::channel();
        let request = Request {
            commmand: cmd,
            write_tx,
            response_tx,
        };
        let result = self.command_sender.try_send(request);
        if let Err(e) = result {
            return Err(Box::new(e));
        }
        if let Err(e) = timeout(self.timeout, write_rx).await {
            return Err(Box::new(e));
        }
        match timeout(self.timeout, response_rx).await {
            Ok(response) => match response {
                Ok(command) => Ok(command),
                Err(e) => Err(Box::new(e)),
            },
            Err(e) => Err(Box::new(e)),
        }
    }
}
