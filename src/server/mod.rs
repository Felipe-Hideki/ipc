#[cfg(feature="async")]
pub mod async_wrapper;

use std::os::unix::net::{ UnixStream, UnixListener };
use std::fs::remove_file;

use logs::{info, error};

use crate::SOCKET_PATH;


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
pub struct RawListener
{
    pub socket_name: String,
    listener: UnixListener
}

impl RawListener
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
            RawListener::handle_client(&callback, stream)?;
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