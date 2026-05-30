use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::ops::Deref;
use std::thread;
use std::thread::{JoinHandle, sleep, spawn};
use std::time::Duration;
use serde_json::json;
//use serde_json::Result;

#[derive(Serialize, Deserialize, Debug)]
struct ClientUDPMessage {
    user_id: u64,
    request_number: u32,
    input_bitmap: u8,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
struct PlayerState {
    x: i32,
    y: i32,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
struct GameState {
    players: std::collections::HashMap<u64, PlayerState>,
}

impl GameState {
    fn new() -> Self {
        GameState {
            players: std::collections::HashMap::new(),
        }
    }
}



#[derive(Serialize, Deserialize, Debug)]
struct ServerUDPMessage {
    request_number: u32,
    state: GameState,
}




//
fn handleClientMessage(client_udpmessage: ClientUDPMessage, game_state: &mut GameState) -> ServerUDPMessage {


    {
        let player_state = game_state.players.get_mut(&client_udpmessage.user_id).unwrap();

        //w is pressed
        if (client_udpmessage.input_bitmap & 1) != 0 {
            player_state.y -= 1;
        }
        if (client_udpmessage.input_bitmap & 2) != 0 {
            player_state.x -= 1;
        }
        if (client_udpmessage.input_bitmap & 4) != 0 {
            player_state.y += 1;
        }
        if (client_udpmessage.input_bitmap & 8) != 0 {
            player_state.x += 1;
        }
    }
    let server_udp = ServerUDPMessage {
        request_number: client_udpmessage.request_number,
        state: game_state.clone(),
    };

    server_udp

}

fn recvMessage() {
    let socket = UdpSocket::bind("127.0.0.1:34254").expect("Could not bind socket");
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.



    let mut buf = [0; 100];


    let mut gameState = GameState::new();

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
               // println!("Received {} bytes from {:?}", size, src);
                println!("{:?}", buf[..size].to_vec());

                let message = serde_json::from_slice::<ClientUDPMessage>(&buf[..size]).unwrap();
                //todo check if message is none

                if (gameState.players.contains_key(&message.user_id) == false) {
                    gameState.players.insert(message.user_id, PlayerState { x: 0, y: 0 });
                }
                let serverUDPMessage = handleClientMessage(message, &mut gameState);
        //        println!("New Game State {:?}, ", serverUDPMessage);

                let jsonMessage = serde_json::to_vec(&serverUDPMessage).expect("Could not serialize");

                socket.send_to(&jsonMessage, &src).expect("Could not send");
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
