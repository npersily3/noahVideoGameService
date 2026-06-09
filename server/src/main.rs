use crossbeam::{channel, select};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Barrier, Mutex, MutexGuard};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use tokio;
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
    left_click: bool,
    mouse_x: u32,
    mouse_y: u32,
    client_perspective: u32,
}

// This the world state without the personal data, (no addresses should be sent)
#[derive(Serialize, Debug)]
struct WorldSnapshot {
    players: HashMap<u64, PlayerState>,
}

#[derive(Serialize, Debug)]
struct ServerUDPMessage {
    request_number: u32,
    server_tick: u32,
    state: WorldSnapshot,
}

// ---------------------------------------------------------------------------
// Stored types — the authoritative server state. A single map keyed by user_id
// holds both the simulation state and the connection bookkeeping, so there is
// one lock and no cross-map lock ordering to maintain. None of this is
// serialized directly; the sender projects it into the wire types above.
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize, Debug)]
struct PlayerState {
    x: i32,
    y: i32,
    angle: f64,
    health: i8,
}

#[derive(Debug, Clone)]
struct Client {
    state: PlayerState,
    socket: SocketAddr,
    last_received_state: u32,
}

#[derive(Debug, Clone)]
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

// Apply a single input bitmap to one player's position and facing angle.
fn apply_input(
    game_state: &mut GameState,
    input_bitmap: u8,
    id: u64,
    mouse_x: u32,
    mouse_y: u32,
    left_click: bool,
    game_perspective: u32,
) {
    {
        let client = game_state.clients.get_mut(&id);

        match client {
            Some(client) => {
                if (input_bitmap & 1) != 0 {
                    client.state.y -= 1;
                }
                if (input_bitmap & 2) != 0 {
                    client.state.x -= 1;
                }
                if (input_bitmap & 4) != 0 {
                    client.state.y += 1;
                }
                if (input_bitmap & 8) != 0 {
                    client.state.x += 1;
                }

                let dy = mouse_y as f64 - client.state.y as f64;
                let dx = mouse_x as f64 - client.state.x as f64;
                client.state.angle = dy.atan2(dx);
            }
            None => {}
        }
    } // &mut client dies here

    if left_click {
        let player_world = 1;
        //now for hit detection
        for (&other_id, other_client) in game_state.clients.iter() {}
    }
}

// Thin socket pump: read datagrams off the wire and hand the raw bytes (plus
// source address) to handle_message over the channel. No parsing or state work
// here, so this thread never blocks on anything but the socket.
fn recv_message(
    socket: Arc<UdpSocket>,
    input_message_channel: channel::Sender<(Vec<u8>, SocketAddr)>,
) {
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.

    let mut buf = [0; 100];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                input_message_channel
                    .send((buf[..size].to_vec(), src))
                    .expect("channel dropped");
            }
            Err(e) => {
                println!("couldn't recieve from {:?}", e);
            }
        }
    }
}

// Pulls raw datagrams off the channel and applies them to the live state. This
// is the old recv_message body, sourcing packets from the channel instead of
// the socket.
fn handle_message(
    world_history: Arc<Mutex<VecDeque<(u32, GameState)>>>,
    input_message_channel: channel::Receiver<(Vec<u8>, SocketAddr)>,
    barrier: Arc<Barrier>,
) {
    // The authoritative "live" state, owned locally by this thread. Inputs are
    // applied here every packet; the snapshot/world_history logic is yours to add.
    let mut current_gamestate = GameState::new();
    let mut game_tick: u32 = 0;

    barrier.wait();

    loop {
        loop {

            let deadline = Instant::now() + TICK_PERIOD;

            // this macro allows us to wait behind 2 blocking operations
            select! {
                recv(input_message_channel) -> msg => {
                    match msg {
                        Ok((buf, src)) => {

                            // Invalid packets are rejected here, before we touch any state.
                            let message = match serde_json::from_slice::<ClientUDPMessage>(&buf) {
                                Ok(message) => message,
                                Err(e) => {
                                    println!("couldn't parse message from {src:?}: {e:?}");
                                    continue;
                                }
                            };

                            // First time we've seen this user: spawn them at the center and seed
                            // last_received_state with this packet's number so the very first
                            // input still counts (and later packets must be strictly newer).
                            let is_new = !current_gamestate.clients.contains_key(&message.user_id);
                            if is_new {
                                current_gamestate.clients.insert(
                                    message.user_id,
                                    Client {
                                        state: PlayerState {
                                            x: 600,
                                            y: 400,
                                            angle: 0.0,
                                            health: 100,
                                        },
                                        socket: src,
                                        last_received_state: message.request_number,
                                    },
                                );
                            }

                            // we have to drop the mutable reference because rust, so this is in its own scope
                            {
                                let client = current_gamestate.clients.get_mut(&message.user_id).unwrap();

                                // drop old packets and update what the newest packet is
                                if !is_new {
                                    if message.request_number <= client.last_received_state {
                                        continue;
                                    }
                                    client.last_received_state = message.request_number;
                                }
                            }

                            apply_input(
                                &mut current_gamestate,
                                message.input_bitmap,
                                message.user_id,
                                message.mouse_x,
                                message.mouse_y,
                                message.left_click,
                                0,
                            );
                        }
                        Err(e) => {
                            panic!("error receiving message from {e:?}");
                        }
                    }
                }
                recv(channel::at(deadline)) -> msg => {
                    break;
                }

            }
        }
        let mut world_history = world_history.lock().unwrap();

        world_history.push_front((game_tick ,current_gamestate.clone()));
        game_tick += 1;
    }
}

fn agones_sdk(barrier: Arc<Barrier>) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let mut sdk = agones::Sdk::new(None, None).await.unwrap();

        barrier.wait();
        sdk.ready().await.unwrap();

        loop {
            let health = sdk.health_check();
            if health.send(()).await.is_err() {
                eprintln!("the health receiver was closed");
            }

            sleep(Duration::from_secs(3))
        }
    });
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

    let barrier = Arc::new(Barrier::new(3));

    let game_history = Arc::new(Mutex::new(VecDeque::<(u32, GameState)>::with_capacity(16)));

    #[cfg(not(feature = "container"))]
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:34254").expect("Could not bind socket"));

    #[cfg(feature = "container")]
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:34254").expect("Could not bind socket"));

    // Raw datagrams flow recv_message -> handle_message over this channel.
    let (input_sender, input_receiver) = channel::unbounded::<(Vec<u8>, SocketAddr)>();

    let s = socket.clone();
    handles.push(spawn(move || recv_message(s, input_sender)));

    let gh = game_history.clone();
    let b = barrier.clone();
    handles.push(spawn(move || handle_message(gh, input_receiver, b)));

    let gh = game_history.clone();
    let s = socket.clone();
    let b = barrier.clone();
    handles.push(spawn(move || sender_thread(gh, s, b)));

    spawn(move || {
        agones_sdk(barrier);
    });

    for handle in handles {
        handle.join().unwrap();
    }
}

// HZ
const TICK_FREQUENCY: u64 = 60;
const TICK_PERIOD: Duration = Duration::from_millis(1000 / TICK_FREQUENCY);

//on a fixed loop, send the head snapshot of the world history to all users.
fn sender_thread(
    world_history: Arc<Mutex<VecDeque<(u32, GameState)>>>,
    socket: Arc<UdpSocket>,
    barrier: Arc<Barrier>,
) {
    barrier.wait();

    loop {
        let starting_time = Instant::now();

        // Build the wire snapshot and collect the target addresses while holding
        // the lock, then release it *before* doing any blocking network sends so
        // recv_message isn't stalled for the whole broadcast.
        let (bytes, targets) = {
            let history = world_history.lock().unwrap();

            // Copy the head of the history. The tick is the u32 coupled with the
            // snapshot, not a counter the sender owns. Empty history => empty
            // snapshot, so the send loop below is a no-op.
            let mut players = HashMap::new();
            let mut targets = Vec::new();
            let mut server_tick = 0;

            if let Some((snapshot_number, game_state)) = history.front() {
                // we know the length, so we can build the hashmap with capacity
                players = HashMap::with_capacity(game_state.clients.len());
                targets = Vec::with_capacity(game_state.clients.len());
                server_tick = *snapshot_number;

                for (&id, client) in game_state.clients.iter() {
                    // Projection: only the PlayerState reaches the wire — socket and
                    // last_received_state stay on the server.
                    players.insert(id, client.state.clone());
                    targets.push(client.socket);
                }
            }

            let message = ServerUDPMessage {
                request_number: 0,
                server_tick,
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

        let delta_time = starting_time.elapsed();
        if delta_time < TICK_PERIOD {
            sleep(TICK_PERIOD - delta_time);
        }
    }
}
