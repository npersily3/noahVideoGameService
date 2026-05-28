use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::thread;
use std::thread::{JoinHandle, sleep, spawn};
use std::time::Duration;
//use serde_json::Result;

#[derive(Serialize, Deserialize, Debug)]
struct ClientUDPMessage {
    user_id: u64,
    request_number: u32,
    inputBitmap: u8,
}

struct PlayerState {
    x: i32,
    y: i32,
}



fn recvMessage() {
    let socket = UdpSocket::bind("127.0.0.1:34254").expect("Could not bind socket");
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.



    let mut buf = [0; 100];


    let player_states: HashMap<usize, PlayerState> = std::collections::HashMap::from([]);
    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, _src)) => {
                println!("Received {} bytes from {:?}", size, _src);
                println!("{:?}", buf[..size].to_vec());

                let message = serde_json::from_slice::<ClientUDPMessage>(&buf[..size]);
                //todo check if message is none
                println!("{:?}", message);

                socket.send_to(&buf[..size], &_src).expect("Could not send");
            }
            Err(e) => {
                println!("couldn't recieve from {:?}", e);
            }
        }
    }


    // println!("Received {} bytes from {:?}", size, add);
}

fn main() {
    //this lets me use windbg JIT
    std::panic::set_hook(Box::new(|info| {
        //  println!("Panic: {info}");
        unsafe {
            core::arch::asm!("int3");
        }
    }));

    let mut handles = Vec::new();

    let handle = spawn(recvMessage);

    handles.push(handle);

    sleep(Duration::from_millis(1));

    for handle in handles {
        handle.join().unwrap();
    }
}
