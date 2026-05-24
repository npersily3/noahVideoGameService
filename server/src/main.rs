use std::net::UdpSocket;
use std::thread;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

fn recvMessage() {
    let socket = UdpSocket::bind("127.0.0.1:34254").expect("Could not bind socket");
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    let mut buf = [0; 100];

    match socket.recv_from(&mut buf) {
        Ok((size, _src)) => {
            println!("Received {} bytes from {:?}", size, _src);


            socket.send_to(&buf[..size], &_src).expect("Could not send");
        }
        Err(e) => {
            println!("couldn't recieve from {:?}", e);
        }
    }


   // println!("Received {} bytes from {:?}", size, add);
}



fn main() {

    let mut handles = Vec::new();


    let handle = spawn(recvMessage);



    handles.push(handle);

    sleep(Duration::from_millis(1));



    for handle in handles {
        handle.join().unwrap();
    }
}
