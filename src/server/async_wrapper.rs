use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::RawListener as IPCServer;

pub struct Connection
{
    stream: UnixStream
}

impl Connection
{
    pub fn new(stream: UnixStream) -> Self
    {
        Self { stream }
    }

    pub async fn read<T>(&mut self) -> Result<T, std::io::Error>
    where T: From<String>
    {
        let mut buf = [0u8; 1024];
        let bytes_read = self.stream.read(&mut buf).await?;
        let message = String::from_utf8_lossy(&buf[..bytes_read]);
        Ok(T::from(message.to_string()))
        // {
        //     command: Commands::from_str(message_parts.next().unwrap()),
        //     data: message_parts.map(|s| s.to_string()).collect()
        // })
    }
    
    pub async fn send(&mut self, response: impl Into<String>) -> Result<(), std::io::Error>
    {
        let response: String = response.into();
        self.stream.write_all(&Vec::from(response)).await
    }
}

pub struct AsyncListener
{
    ipc_server: IPCServer
}

impl AsyncListener
{
    pub fn new(sock_name: &str) -> Result<Self, std::io::Error>
    {
        Ok(Self { 
            ipc_server: IPCServer::new(sock_name)?
        })
    }

    pub fn get_raw_path(&self) -> String
    {
        self.ipc_server.raw_path.to_string()
    }
    /// Waits for a connection to be established
    /// 
    /// Returns a Connection object if a connection is established, otherwise None
    /// 
    /// It will panic if the UnixStream cannot be created from the std::net::UnixStream or
    /// if the UnixStream cannot be set to non-blocking
    pub async fn wait_for_connection(&mut self) -> Option<Connection>
    {
        self.ipc_server.wait_connection().ok().map(|data|
        {
            data.stream.set_nonblocking(true).expect("Error setting UnixStream to non-blocking");
            Connection::new(UnixStream::from_std(data.stream).expect("Error creating UnixStream from std::net::UnixStream"))
        })
    }
}