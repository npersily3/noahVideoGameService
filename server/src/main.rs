use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
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
    players: HashMap<u64, PlayerState>,
}

impl GameState {
    fn new() -> Self {
        GameState {
            players: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerUDPMessage {
    request_number: u32,
    state: GameState,
}

struct ClientData {
    socket: SocketAddr,
}
struct ServerData {
    clients: HashMap<u64, ClientData>,
}
impl ServerData {
    fn new() -> Self {
        ServerData {
            clients: HashMap::new(),
        }
    }
}

//update gamestate based on client message`
fn handle_client_message(
    client_udpmessage: ClientUDPMessage,
    game_state: &mut RwLockWriteGuard<GameState>,
) {
    let player_state = game_state
        .players
        .get_mut(&client_udpmessage.user_id)
        .unwrap();

    //w is pressed
    if (client_udpmessage.input_bitmap & 1) != 0 {
        player_state.y -= 1;
    }
    //a
    if (client_udpmessage.input_bitmap & 2) != 0 {
        player_state.x -= 1;
    }
    //s
    if (client_udpmessage.input_bitmap & 4) != 0 {
        player_state.y += 1;
    }
    //d
    if (client_udpmessage.input_bitmap & 8) != 0 {
        player_state.x += 1;
    }
}

fn recv_message(
    game_state: Arc<RwLock<GameState>>,
    server_data: Arc<RwLock<ServerData>>,
    socket: Arc<UdpSocket>,
) {
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.

    let mut buf = [0; 100];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                let message = serde_json::from_slice::<ClientUDPMessage>(&buf[..size]).unwrap();

                // new scope to for rwLock on game state
                {
                    let mut local_game_state = game_state.write().unwrap();

                    if !local_game_state.players.contains_key(&message.user_id)   {
                        local_game_state
                            .players
                            .insert(message.user_id, PlayerState { x: 0, y: 0 });

                        //now we have to lock and insert the address into global server data then we drop it after the scope
                        {
                            let mut local_server_data = server_data.write().unwrap();
                            local_server_data
                                .clients
                                .insert(message.user_id, ClientData { socket: src });
                        }
                    }

                    handle_client_message(message, &mut local_game_state);
                    //        println!("New Game State {:?}, ", serverUDPMessage);
                }
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
        println!("Panic: {info}");
        unsafe {
            core::arch::asm!("int3");
        }
    }));

    let mut handles = Vec::new();

    let server_data = Arc::new(RwLock::new(ServerData::new()));
    let game_state = Arc::new(RwLock::new(GameState::new()));
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:34254").expect("Could not bind socket"));

    let sd = server_data.clone();
    let gs = game_state.clone();
    let s = socket.clone();

    let handle = spawn(move || recv_message(gs, sd, s));

    handles.push(handle);

    let handle =
        spawn(move || sender_thread(game_state.clone(), server_data.clone(), socket.clone()));

    handles.push(handle);

    for handle in handles {
        handle.join().unwrap();
    }
}


//30 HZ
const TICK_FREQUENCY: u64 = 30;
const TICK_PERIOD: Duration = Duration::from_millis(1/TICK_FREQUENCY);

//on a fixed loop, send full gamestate to all users.
// I need a user database that has their conn
fn sender_thread(
    game_state: Arc<RwLock<GameState>>,
    server_data: Arc<RwLock<ServerData>>,
    socket: Arc<UdpSocket>,
) {
    //figure out a way
    loop {
        let starting_time  = Instant::now();
        {
            let server_data = server_data.read().unwrap();

            let json_message;

            //lock and make
            {
                let local_game_state = game_state.read().unwrap();
                let server_udp = ServerUDPMessage {
                    request_number: 0,
                    state: local_game_state.clone(),
                };

                json_message = serde_json::to_vec(&server_udp).expect("Could not serialize");
            }

            for client in server_data.clients.values() {
                socket
                    .send_to(&json_message, client.socket)
                    .expect("Could not send");
            }

        }
        let delta_time = starting_time.elapsed();
        if delta_time < TICK_PERIOD {
            sleep(TICK_PERIOD - delta_time);
        }
    }

}
