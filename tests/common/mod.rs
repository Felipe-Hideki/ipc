use ipc::{SOCKET_PATH, StreamData};

use std::fs;
use std::{io, io::Write};

pub fn setup()
{
    match fs::create_dir_all(SOCKET_PATH)
    {
        Ok(_) => { },
        Err(e) => 
        {
            match e.kind()
            {
                io::ErrorKind::AlreadyExists => { },
                io::ErrorKind::PermissionDenied => panic!("Permission denied to create directory!"),
                _ => panic!("Failed to create directory: {:?}", e)
            }
        }
    }
}

pub fn callback_fn(data: &[u8], stream_data: &mut StreamData) 
{
    let message = String::from_utf8_lossy(data);
    assert_eq!(message, "Hello, ");
    print!("{}", message);
    stream_data.stream.write_all("World!".as_bytes()).unwrap();
}
