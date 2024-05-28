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
use std::os::unix::net::{ UnixStream, UnixListener };
use std::io::{ Read, Write };
use std::fs::remove_file;
use std::time::Duration;

use logs::{info, error};

/// Default path for the sockets
/// 
/// Used when the user doesn't provide a full path for the socket.
pub const SOCKET_PATH: &str = "/var/lib/tag_connectivity/sockets";

/// Data received from the server, it currently only holds the [UnixStream] but it can be expanded to hold more data. 
#[derive(Debug)]
pub struct StreamData
{
    pub stream: UnixStream
//  ...
}

/// Server side communication handler, it is capable of listening for incoming connections and handling the data received.
///
/// It provides two methods to handle the incoming data, these two only differentiates in the way the data is handled. 
/// [`listen`](Server::listen) has an internal loop, while 
/// [`wait_connection`](Server::wait_connection) returns the stream.
#[derive(Debug)]
pub struct Server
{
    pub socket_name: String,
    listener: UnixListener
}

impl Server
{
    pub fn new(socket_name: &str) -> Result<Self, std::io::Error>
    {
        let server_path = if socket_name.as_bytes()[0] == b'/'
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
            socket_name: socket_name.to_string(),
            listener: UnixListener::bind(server_path)?
        })
    }

    /// Listens for incoming connections and calls the `callback` function to handle the data received.
    /// 
    /// It is a blocking method that waits for incoming connections and calls the callback function 
    /// to handle the data received in a loop.
    /// 
    /// # Example
    /// ```
    /// use std::{io, io::Write};
    /// 
    /// use ipc::{Server, StreamData};
    /// 
    /// fn handle_data(stream_data: &mut StreamData)
    /// {
    ///     let buf = &mut vec![0u8; 512];
    ///     let msg_len = stream_data.stream.read(buf).unwrap();
    ///     let msg = format!("Received {:?}", String::from_utf8_lossy(&buf[..msg_len]));
    ///     stream_data.stream.write_all(msg.as_bytes()).unwrap();
    /// }
    /// 
    /// fn flow(mut server: Server) -> Result<(), io::Error>
    /// {
    ///     server.listen(handle_data)
    /// }
    pub fn listen(&mut self, callback: impl Fn(&mut StreamData)) -> Result<(), std::io::Error>
    {
        for stream in self.listener.incoming()
        {
            let stream = stream?;
            Server::handle_client(&callback, stream)?;
        }
        Ok(())
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
    pub fn wait_connection(&mut self) -> Result<StreamData, std::io::Error>
    {
        let (stream, _) = self.listener.accept()?;
        
        Ok(StreamData { stream })
    }

    /// Handles the client connection, calling the callback function.
    /// 
    /// A helper function used by [`listen`](Server::listen).
    fn handle_client(callback: &impl Fn(&mut StreamData), 
                    stream: UnixStream) -> Result<(), std::io::Error>
    {
        let mut stream_data: StreamData = StreamData 
        {
            stream
        };

        callback(&mut stream_data);
        Ok(())   
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

/// Client side communication handler, provides a simple interface to send data to a server.
/// 
/// It provides two methods to send data: 
/// 
/// [`send`](Client::send) -> Need to open the connection beforehand and can be used to send multiple messages in a 
/// short period of time without the overhead of creating a new connection every call.
/// 
/// [`send_one_shot`](Client::send_one_shot) -> Create and delete the connection every time it is called, it is supposed
/// to be used when the message doesn't need to be sent multiple times in a short period of time.
#[derive(Debug)]
pub struct Client
{
    connection: UnixStream
}

impl Client
{
    pub fn new(path: impl Into<String>, read_timeout: Option<Duration>, write_timeout: Option<Duration>) -> Result<Self, std::io::Error>
    {
        let path: String = path.into();
        let connection = if path.as_bytes()[0] != b'/'
        {
            UnixStream::connect(format!("{}/{}", SOCKET_PATH, path))?
        }
        else 
        {
            UnixStream::connect(path)?
        };
        
        connection.set_read_timeout(read_timeout)?;
        connection.set_write_timeout(write_timeout)?;
        Ok(Self
        {
            connection
        })
    }

    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error>
    {
        self.connection.set_read_timeout(timeout)
    }

    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) -> Result<(), std::io::Error>
    {
        self.connection.set_write_timeout(timeout)
    }

    /// Sends data to the server.
    /// 
    /// It maintains the connection open and can be used to send multiple messages in a short period
    /// of time without the overhead of creating a new connection every time.
    /// 
    /// If the message doesn't need to be sent multiple times in a short period of time, it is recommended
    /// to use the [`send_one_shot`](Client::send_one_shot) method instead.
    pub fn send(&mut self, data: &[u8]) -> Result<(), std::io::Error>
    {
        self.connection.write_all(data)
    }

    /// Reads data from the server.
    /// 
    /// It reads data from the server, it is supposed to be used after the [`send`](Client::send) method.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error>
    {
        self.connection.read(buf)
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
}