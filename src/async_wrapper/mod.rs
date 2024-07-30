use std::fs::remove_file;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;

use logs::info;

use crate::get_full_path;

pub struct AsyncConnection {
    stream: tokio::net::UnixStream,
}

impl AsyncConnection {
    pub fn new(stream: tokio::net::UnixStream) -> Self {
        Self { stream }
    }

    pub async fn read_raw(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self.stream.read(buf).await {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "No data read",
                    ));
                }
                Ok(bytes_read)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn read<T>(&mut self) -> Result<T, std::io::Error>
    where
        T: From<String>,
    {
        let mut buf = [0u8; 1024];
        let bytes_read = self.stream.read(&mut buf).await?;
        if bytes_read == 0
        // as per the tokio docs, a return value of 0 might indicate that the stream has been closed
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No data read",
            ));
        }

        let message = String::from_utf8_lossy(&buf[..bytes_read]);
        Ok(T::from(message.to_string()))
        // {
        //     command: Commands::from_str(message_parts.next().unwrap()),
        //     data: message_parts.map(|s| s.to_string()).collect()
        // })
    }

    pub async fn send(&mut self, response: impl Into<String>) -> Result<(), std::io::Error> {
        let response: String = response.into();
        self.stream.write_all(&Vec::from(response)).await
    }
}

pub struct AsyncServer {
    listener: tokio::net::UnixListener,
    raw_path: String,
}

impl AsyncServer {
    pub fn new(sock_name: &str) -> Result<Self, std::io::Error> {
        let full_path = get_full_path(sock_name);
        info!("Binding to socket: {}", full_path);
        match remove_file(&full_path) {
            Ok(_) => {
                info!("Removed existing socket file: {}", full_path);
            }
            Err(_) => {}
        }
        Ok(Self {
            listener: UnixListener::bind(full_path)?,
            raw_path: sock_name.to_string(),
        })
    }

    pub fn get_raw_path(&self) -> String {
        self.raw_path.to_string()
    }
    /// Waits for a connection to be established
    ///
    /// Returns a Connection object if a connection is established, otherwise None
    ///
    /// It will panic if the tokio::net::UnixStream cannot be created from the std::net::tokio::net::UnixStream or
    /// if the tokio::net::UnixStream cannot be set to non-blocking
    pub async fn wait_for_connection(&mut self) -> Option<AsyncConnection> {
        self.listener
            .accept()
            .await
            .map(|data| AsyncConnection::new(data.0))
            .ok()
    }
}

pub async fn new_client_async(sock_name: &str) -> Result<AsyncConnection, std::io::Error> {
    let full_path = get_full_path(sock_name);

    info!("Connecting to socket: {}", full_path);
    let stream = tokio::net::UnixStream::connect(full_path).await?;
    Ok(AsyncConnection::new(stream))
}
