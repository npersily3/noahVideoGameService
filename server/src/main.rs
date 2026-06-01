use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
//use serde_json::Result;




//Types that are read over the wire
#[derive(Serialize, Deserialize, Debug)]
struct ClientUDPMessage {
    user_id: u64,
    request_number: u32,
    input_bitmap: u8,
}
#[derive(Serialize, Deserialize, Debug)]
struct ServerUDPMessage {
    request_number: u32,
    state: GameState,
}




// player state, soon to add position position and other stuff
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




// Types only the server knows about
struct ClientData {
    socket: SocketAddr,
    last_received_state: u32
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

    //TODO chuck it if it is an old request/update the request number
    // make sure to also chuck duplicates
    //if(client_udpmessage.request_number < )




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


// this functions just updates the world after pulling inputs off the wire
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

            // if we succesfully recieved the packet
            Ok((size, src)) => {

                // convert it from a json to our matching struct
                let message = match serde_json::from_slice::<ClientUDPMessage>(&buf[..size]) {
                    Ok(message) => message,
                    Err(e) => {
                        println!("couldn't parse message from {src:?}: {e:?}");
                        continue;
                    }
                };

                // new scope to for rwLock on game state
                //TODO this is a complicated thing, I need to maintain lock ordering while making sure handle client message gets both things
                // i think the answer is to first read lock the server data, then write if neccesary, then lock the game state for the right,
                // really this whole locking thing does not matter that much because I really have one thread
                {
                    let mut local_game_state = game_state.write().unwrap();

                    if !local_game_state.players.contains_key(&message.user_id)   {
                        local_game_state.players.insert(message.user_id, PlayerState { x: 600, y:400 });

                        //now we have to lock and insert the address into global server data then we drop it after the scope
                        {
                            let mut local_server_data = server_data.write().unwrap();
                            local_server_data
                                .clients
                                .insert(message.user_id, ClientData { socket: src, last_received_state: 0 });
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
    //this lets me use windbg JIT, not neccesary for users
    std::panic::set_hook(Box::new(|info| {
        println!("Panic: {info}");
        unsafe {
            core::arch::asm!("int3");
        }
    }));

    let mut handles = Vec::new();

    // We need arc rwlocks because multiple people own and read write from these two data sources
    //The lock order is server data then game state, which I think intuitively makes sense
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


// HZ
const TICK_FREQUENCY: u64 = 60;
const TICK_PERIOD: Duration = Duration::from_millis(1000/TICK_FREQUENCY);

// This function sends the game state to all users.
fn sender_thread(
    game_state: Arc<RwLock<GameState>>,
    server_data: Arc<RwLock<ServerData>>,
    socket: Arc<UdpSocket>,
) {
    //figure out a way
    loop {
        let starting_time  = Instant::now();
        {
            let json_message;

            //lock the game state and convert it to a local json so the game can be updated, but all the other users will get the same game
            {
                let local_game_state = game_state.read().unwrap();
                let server_udp = ServerUDPMessage {
                    request_number: 0,
                    state: local_game_state.clone(),
                };

               // println!("Server UDP Request: {:?}", server_udp);

                json_message = serde_json::to_vec(&server_udp).expect("Could not serialize");
            }

            // now that we have the game state, we send it to everybody
            let server_data_local = server_data.read().unwrap();

            for client in server_data_local.clients.values() {
                if let Err(e) = socket.send_to(&json_message, client.socket) {
                    println!("couldn't send to {:?}: {e:?}", client.socket);
                    continue;
                }

               // println!("Sent udp message to client: {:?}", client.socket);
            }

        }
        // wait until the current tick is over,
        let delta_time = starting_time.elapsed();
        if delta_time < TICK_PERIOD {
            sleep(TICK_PERIOD - delta_time);
        }
    }

}
