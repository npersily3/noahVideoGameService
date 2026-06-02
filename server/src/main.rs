use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
//use serde_json::Result;

// ---------------------------------------------------------------------------
// Wire types — what actually goes over the socket. Keep these separate from the
// stored types below so connection data (addresses, sequence numbers) can never
// leak into a broadcast and we only serialize the bytes we mean to send.
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ClientUDPMessage {
    user_id: u64,
    request_number: u32,
    input_bitmap: u8,
}

// Public, per-player view that is safe to broadcast to everyone.
#[derive(Serialize, Debug)]
struct PlayerSnapshot {
    x: i32,
    y: i32,
}

// The world as seen on the wire. Same JSON shape as before
// (`{ "players": { "<id>": { "x", "y" } } }`) so the Go client is unchanged.
#[derive(Serialize, Debug)]
struct WorldSnapshot {
    players: HashMap<u64, PlayerSnapshot>,
}

#[derive(Serialize, Debug)]
struct ServerUDPMessage {
    request_number: u32,
    state: WorldSnapshot,
}

// ---------------------------------------------------------------------------
// Stored types — the authoritative server state. A single map keyed by user_id
// holds both the simulation state and the connection bookkeeping, so there is
// one lock and no cross-map lock ordering to maintain. None of this is
// serialized directly; the sender projects it into the wire types above.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct PlayerState {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Client {
    state: PlayerState,
    socket: SocketAddr,
    last_received_state: u32,
}

#[derive(Debug)]
struct GameState {
    clients: HashMap<u64, Client>,
}

impl GameState {
    fn new() -> Self {
        GameState {
            clients: HashMap::new(),
        }
    }
}

// Apply a single input bitmap to one player's position.
fn apply_input(player: &mut PlayerState, input_bitmap: u8) {
    //w is pressed
    if (input_bitmap & 1) != 0 {
        player.y -= 1;
    }
    //a
    if (input_bitmap & 2) != 0 {
        player.x -= 1;
    }
    //s
    if (input_bitmap & 4) != 0 {
        player.y += 1;
    }
    //d
    if (input_bitmap & 8) != 0 {
        player.x += 1;
    }
}

fn recv_message(game_state: Arc<Mutex<GameState>>, socket: Arc<UdpSocket>) {
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.

    let mut buf = [0; 100];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                // Invalid packets are rejected here, before we ever take the lock.
                let message = match serde_json::from_slice::<ClientUDPMessage>(&buf[..size]) {
                    Ok(message) => message,
                    Err(e) => {
                        println!("couldn't parse message from {src:?}: {e:?}");
                        continue;
                    }
                };

                let mut state = game_state.lock().unwrap();

                // First time we've seen this user: spawn them at the center and seed
                // last_received_state with this packet's number so the very first
                // input still counts (and later packets must be strictly newer).
                let is_new = !state.clients.contains_key(&message.user_id);
                if is_new {
                    state.clients.insert(
                        message.user_id,
                        Client {
                            state: PlayerState { x: 600, y: 400 },
                            socket: src,
                            last_received_state: message.request_number,
                        },
                    );
                }

                let client = state.clients.get_mut(&message.user_id).unwrap();

                if !is_new {
                    // Basic sequencing (goal #6): drop stale or duplicate packets.
                    // NOTE: no u32 wraparound handling yet — fine until ~2 years at 60Hz.
                    if message.request_number <= client.last_received_state {
                        continue;
                    }
                    client.last_received_state = message.request_number;
                }

                apply_input(&mut client.state, message.input_bitmap);
            }
            Err(e) => {
                println!("couldn't recieve from {:?}", e);
            }
        }
    }
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

    let game_state = Arc::new(Mutex::new(GameState::new()));


   #[cfg(not(feature = "container"))]
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:34254").expect("Could not bind socket"));

    #[cfg(feature = "container")]
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:34254").expect("Could not bind socket"));

    let gs = game_state.clone();
    let s = socket.clone();
    handles.push(spawn(move || recv_message(gs, s)));

    let gs = game_state.clone();
    let s = socket.clone();
    handles.push(spawn(move || sender_thread(gs, s)));

    for handle in handles {
        handle.join().unwrap();
    }
}

// HZ
const TICK_FREQUENCY: u64 = 60;
const TICK_PERIOD: Duration = Duration::from_millis(1000 / TICK_FREQUENCY);

//on a fixed loop, send full gamestate to all users.
fn sender_thread(game_state: Arc<Mutex<GameState>>, socket: Arc<UdpSocket>) {
    // Monotonic snapshot number so clients can sequence the state they receive.
    let mut tick: u32 = 0;

    loop {
        let starting_time = Instant::now();

        // Build the wire snapshot and collect the target addresses while holding
        // the lock, then release it *before* doing any blocking network sends so
        // recv_message isn't stalled for the whole broadcast.
        let (bytes, targets) = {
            let state = game_state.lock().unwrap();

            // we know the length, so we can build the hashmap
            let mut players = HashMap::with_capacity(state.clients.len());
            let mut targets = Vec::with_capacity(state.clients.len());


            for (&id, client) in state.clients.iter() {
                // Projection: only x/y reach the wire — socket and
                // last_received_state stay on the server.
                players.insert(
                    id,
                    PlayerSnapshot {
                        x: client.state.x,
                        y: client.state.y,
                    },
                );
                targets.push(client.socket);
            }

            let message = ServerUDPMessage {
                request_number: tick,
                state: WorldSnapshot { players },
            };

            let bytes = serde_json::to_vec(&message).expect("Could not serialize snapshot");
            (bytes, targets)
        };

        for addr in &targets {
            if let Err(e) = socket.send_to(&bytes, addr) {
                println!("couldn't send to {addr:?}: {e:?}");
            }
        }

        tick = tick.wrapping_add(1);

        let delta_time = starting_time.elapsed();
        if delta_time < TICK_PERIOD {
            sleep(TICK_PERIOD - delta_time);
        }
    }
}
