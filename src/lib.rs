//! Library for inter-process communication using Unix sockets, it provides a simple 
//! interface to send and receive data between processes.
//! 
//! # Example
//! ```
//! use std::thread::spawn;
//! use std::io;
//! 
//! use ipc::{Server, Client, ResponseOption};
//! 
//! fn main() -> Result<(), io::Error>
//! {
//!     let sock_path = "basic.sock";
//!     let mut server = Server::new(sock_path)?;
//!
//!     let listener = move ||
//!     {
//!         let stream_data = server.wait_connection()?;
//! 
//!         let mut buf = vec![0u8; 512];
//!         let msg_len = stream_data.stream.read(&mut buf)?;
//! 
//!         let message = String::from_utf8_lossy(&buf[..msg_len]);
//!         
//!         assert_eq!(msg_len, 13);
//!         assert_eq!(message, "Hello, world!");
//! 
//!         println!("Received {:?}", String::from_utf8_lossy(&buf));
//!         Ok::<(), io::Error>(())
//!     };
//!
//!     spawn(listener);
//!
//!     Client::send_one_shot(b"Hello, world!", sock_path, ResponseOption::DontWaitForResponse)?;
//!     Ok(())
//! }
//! ```
#[cfg(feature="async")]
pub mod async_wrapper;

use std::fs::create_dir_all;
use std::os::unix::net::{UnixStream, UnixListener};
use std::io::{ Read, Write };
use std::path::Path;
use std::time::Duration;
use std::fs::remove_file;

use logs::{info, error};

/// Default path for the sockets
/// 
/// Used when the user doesn't provide a full path for the socket.
pub const SOCKET_PATH: &str = "/tmp";

fn get_full_path(path: impl Into<String>) -> String
{
    let path: String = path.into();
    if path.as_bytes()[0] != b'/'
    {
        format!("{}/{}", SOCKET_PATH, path)
    }
    else 
    {
        path
    }
}
/// Response option when sending data.
/// 
/// Used to specify if the client should wait for a response from the server or not when using the 
/// [`send_one_shot`](Client::send_one_shot) method. 
/// 
/// Note: If [`ResponseOption::WaitForResponse`] is used, the buffer must be big enough to hold the response,
/// otherwise the response will be truncated.
/// 
/// # Example
/// ```
/// use std::{io, io::Write};
/// 
/// use ipc::{Client, ResponseOption};
/// 
/// fn send_and_wait(msg: String) -> Result<String, io::Error>
/// {
///     let mut buf = vec![0u8; 512];
///     let msg_size = Client::send_one_shot(msg.as_bytes(), "basic.sock", ResponseOption::WaitForResponse(&mut buf))?;
///     let response = String::from_utf8_lossy(&buf[..msg_size]);
///     Ok(response.to_string())
/// }
/// ```
pub enum ResponseOption<'a>
{
    WaitForResponse(&'a mut Vec<u8>),
    DontWaitForResponse
}

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

    pub fn read_raw(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error>
    {
        self.stream.read(buf)
    }

    pub fn read<T>(&mut self) -> Result<T, std::io::Error>
    where T: From<String>
    {
        let mut buf = [0u8; 1024];
        let bytes_read = self.stream.read(&mut buf)?;
        if bytes_read == 0
        {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No data read"));
        }

        let message = String::from_utf8_lossy(&buf[..bytes_read]);
        Ok(T::from(message.to_string()))
    }
    
    pub fn send_raw(&mut self, response: Vec<u8>) -> Result<(), std::io::Error>
    {
        self.stream.write_all(&response.as_slice())
    } 

    pub fn send(&mut self, response: impl Into<String>) -> Result<(), std::io::Error>
    {
        let response: String = response.into();
        self.stream.write_all(&Vec::from(response))
    }
}

pub fn new_client(path: impl Into<String>) -> Result<UnixStream, std::io::Error>
{
    UnixStream::connect(get_full_path(path))
}

/// Sends data to the server and waits for a response if needed. 
/// 
/// It cant use the connection created when the client is instantiated, it is supposed to be used when
/// the message doesn't need to be sent multiple times in a short period of time.
/// 
/// If the message needs to be sent multiple times in a short period of time, it is recommended to use
/// the [`send`](Client::send) method instead.
/// 
/// # Example
/// ```
/// use std::{io, io::Write};
/// 
/// use ipc::{Client, ResponseOption};
/// 
/// fn send_msg(sock: String, msg: String) -> Result<(), io::Error>
/// {
///     Client::send_one_shot(msg.as_bytes(), sock, ResponseOption::DontWaitForResponse)?
/// }
/// ```
pub fn send_one_shot(data: &[u8], 
                    path: impl Into<String>, 
                    wait_for_response: ResponseOption) -> Result<usize, std::io::Error>
{
    let mut con = UnixStream::connect(format!("{}/{}", SOCKET_PATH, path.into()))?; 
    con.write_all(data)?;
    match wait_for_response
    {
        ResponseOption::WaitForResponse(mut buf) =>
        {
            Ok(con.read(&mut buf)?)
        },
        ResponseOption::DontWaitForResponse =>
        {
            Ok(0)
        }
    }
}


/// Server side communication handler, it is capable of listening for incoming connections and handling the data received.
///
/// It provides two methods to handle the incoming data, these two only differentiates in the way the data is handled. 
/// [`listen`](Server::listen) has an internal loop, while 
/// [`wait_connection`](Server::wait_connection) returns the stream.
#[derive(Debug)]
pub struct Server
{
    pub raw_path: String,
    pub socket_name: String,
    listener: UnixListener
}

impl Server
{
    pub fn new(socket_name: &str) -> Result<Self, std::io::Error>
    {
        let server_path = if socket_name.starts_with('/')
        {
            socket_name.to_string()
        }
        else
        {
            format!("{SOCKET_PATH}/{socket_name}")
        };
        info!("Creating server {:?}", server_path);
        
        match remove_file(&server_path)
        {
            Ok(_) => { },
            Err(_) => 
            {
                error!("Socket not found, creating a new one...");
            }
        }
        Ok(Self
        {
            raw_path: server_path.to_string(),
            socket_name: socket_name.to_string(),
            listener: UnixListener::bind(server_path)?
        })
    }

    /// Waits for a connection and returns the stream and the data received.
    /// 
    /// Returns the stream and the data received after an incoming connection is established, 
    /// allowing the user to handle the data.
    /// 
    /// # Example
    /// ```
    /// use std::{io, io::Write};
    /// 
    /// use ipc::{Server, StreamData};
    /// 
    /// fn handle_data(data: &[u8], mut stream_data: StreamData)
    /// {
    ///     let msg = format!("Received {:?}", String::from_utf8_lossy(data));
    ///     stream_data.stream.write_all(msg.as_bytes()).unwrap();
    /// }
    /// 
    /// fn flow(mut server: Server) -> Result<(), io::Error>
    /// {
    ///     let mut buf = vec![0u8; 512];
    ///     loop
    ///     {
    ///         let mut stream_data = server.wait_connection()?;
    ///     
    ///         let msg_len = stream_data.stream.read(&mut buf)?;
    /// 
    ///         handle_data(&buf[..msg_len], stream_data);
    ///     }
    /// }
    pub fn wait_connection(&mut self) -> Result<Connection, std::io::Error>
    {
        let (stream, _) = self.listener.accept()?;
        
        Ok(Connection::new(stream))
    }
}