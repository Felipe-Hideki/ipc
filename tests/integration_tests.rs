// mod common;

// use ipc::*;

// use std::thread::spawn;
// use std::{io, io::Write};


// #[test]
// fn callback_with_fn() -> Result<(), io::Error>
// {
//     common::setup();

//     let sock_path = "callback_fn.sock";
//     let mut server = Server::new(sock_path)?;
//     let mut client = Client::new(sock_path)?;

//     spawn(move || server.listen(512, common::callback_fn));

//     let mut client_buf = vec![0u8; 512];
//     let msg_length = client.send("Hello, ".as_bytes(), ResponseOption::WaitForResponse(&mut client_buf))?;
    
//     let response = String::from_utf8_lossy(&client_buf[..msg_length]);
//     assert_eq!(response, "World!");
//     print!("{}", response);
//     Ok(())
// }

// #[test]
// fn callback_with_closure() -> Result<(), io::Error>
// {
//     common::setup();

//     let sock_path = "callback_closure.sock";
//     let mut server = Server::new(sock_path)?;
//     let mut client = Client::new(sock_path)?;

//     let listener = move |data: &[u8], stream_data: &mut StreamData|
//     {
//         let message = String::from_utf8_lossy(data);
//         assert_eq!(message, "Hello, ");
//         print!("{}", message);
//         stream_data.stream.write_all("World!".as_bytes()).unwrap();
//     };

//     spawn(move || server.listen(512, listener));

//     let mut client_buf = vec![0u8; 512];
//     let msg_length = client.send("Hello, ".as_bytes(), ResponseOption::WaitForResponse(&mut client_buf))?;
    
//     let response = String::from_utf8_lossy(&client_buf[..msg_length]);
//     assert_eq!(response, "World!");

//     print!("{}", response);
//     Ok(())
// }

// #[test]
// fn wait_connection() -> Result<(), io::Error>
// {
//     common::setup();

//     let sock_path = "listen_test.sock";
//     let mut server = Server::new(sock_path)?;
//     let mut client = Client::new(sock_path)?;

//     let listener = move ||
//     {
//         let mut buf = vec![0u8; 512];
//         let (msg_len, _stream_data) = server.wait_connection(&mut buf)?;
        
//         let message = String::from_utf8_lossy(&buf[..msg_len]);
        
//         assert_eq!(message, "Hello, world!");
        
//         print!("{}", message);
//         Ok::<(), io::Error>(())
//     };

//     spawn(listener);

//     client.send("Hello, world!".as_bytes(), ResponseOption::DontWaitForResponse)?;
//     Ok(())
// }

// #[test]
// fn wait_response() -> Result<(), io::Error>
// {
//     common::setup();

//     let sock_path = "response_test.sock";
//     let mut server = Server::new(sock_path)?;
//     let mut client = Client::new(sock_path)?;

//     let listener = move ||
//     {
//         let mut buf = vec![0u8; 512];
//         let (msg_len, mut stream_data) = server.wait_connection(&mut buf)?;
//         let message = String::from_utf8_lossy(&buf[..msg_len]).into_owned();
        
//         assert_eq!(message, "Hello, ");
        
//         print!("{}", message);

//         stream_data.stream.write_all("World!".as_bytes())?;
//         Ok::<(), io::Error>(())
//     };

//     spawn(listener);

//     let mut client_buf = vec![0u8; 512];
//     let msg_length = client.send("Hello, ".as_bytes(), ResponseOption::WaitForResponse(&mut client_buf))?;
    
//     let response = String::from_utf8_lossy(&client_buf[..msg_length]);

//     assert_eq!(response, "World!");
//     print!("{}", response);
//     Ok(())
// }

// #[test]
// fn send_one_shot() -> Result<(), io::Error>
// {
//     common::setup();

//     let sock_path = "one_shot_test.sock";
//     let mut server = Server::new(sock_path)?;

//     let listener = move ||
//     {
//         let mut buf = vec![0u8; 512];
//         let (msg_len, mut stream_data) = server.wait_connection(&mut buf)?;
//         let message = String::from_utf8_lossy(&buf[..msg_len]).into_owned();
        
//         assert_eq!(message, "Hello, ");
        
//         print!("{}", message);

//         stream_data.stream.write_all("World!".as_bytes())?;
//         Ok::<(), io::Error>(())
//     };

//     spawn(listener);

//     let mut buf = vec![0u8; 512];

//     let msg_length = Client::send_one_shot("Hello, ".as_bytes(), 
//         sock_path, 
//         ResponseOption::WaitForResponse(&mut buf))?;
    
//     let response = String::from_utf8_lossy(&buf[..msg_length]);

//     assert_eq!(response, "World!");
//     print!("{}", response);
//     Ok(())
// }