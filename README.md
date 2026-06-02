# noahVideoGameServer

A from-scratch authoritative multiplayer game server, built as a learning project.
The goal is to learn **UDP networking, authoritative netcode, and Kubernetes** by
building a real-time game where dots move around a shared canvas, driven entirely by
a server tick loop.

This is intentionally built up in phases — from a bare local UDP loop all the way to
a geo-distributed, autoscaling fleet on Kubernetes — so each networking and infra
concept is learned one layer at a time.

---

## Architecture

```
 Browser (canvas + WASD)          Go client                 Rust server
 ┌─────────────────────┐  WS    ┌──────────────┐   UDP    ┌──────────────────┐
 │ index.html          │◄──────►│ main.go      │◄────────►│ src/main.rs      │
 │  - renders players  │        │  - WS bridge │ :34254   │  - owns state    │
 │  - captures input   │        │  - UDP relay │          │  - 60 Hz tick    │
 └─────────────────────┘        └──────────────┘          └──────────────────┘
```

- **Browser (`client/index.html`)** — draws every player as a dot on a 1200×800
  canvas, captures `WASD` keypresses, and talks to its Go client over a WebSocket.
- **Go client (`client/`)** — serves the HTML page, bridges the browser WebSocket to
  the server's UDP socket. It batches input into a bitmap and sends one packet per
  tick (60 Hz), and forwards server snapshots back to the browser.
- **Rust server (`server/`)** — the authority. It receives client input, updates the
  game state, and broadcasts the full game state to every connected client on a fixed
  60 Hz tick.

### Wire protocol (current)

Messages are JSON (will be replaced with a binary format in a later phase).

**Client → Server** (`ClientUDPMessage`):
```json
{ "user_id": 12345, "request_number": 42, "input_bitmap": 9 }
```
`input_bitmap` is `W=1, A=2, S=4, D=8` OR'd together (e.g. `9` = W+D).

**Server → Client** (`ServerUDPMessage`):
```json
{ "request_number": 0, "state": { "players": { "12345": { "x": 600, "y": 400 } } } }
```

---

## Running locally

You need **Rust** (edition 2024) and **Go** installed.

### 1. Start the server

```powershell
cd server
cargo run
```

The server binds UDP on `127.0.0.1:34254` and starts ticking at 60 Hz.

### 2. Start one or more clients ("gamers")

The easiest way is the helper script, which builds the client and launches N
instances, each on its own HTTP port, opening a browser tab for each:

```powershell
cd client
./launch_gamers.ps1 -N 3            # gamers on 8080, 8081, 8082
./launch_gamers.ps1 -N 5 -BasePort 9000
./launch_gamers.ps1 -N 2 -NoBrowser # don't auto-open tabs
```

Or run a single client by hand:

```powershell
cd client
go build -o client.exe .
./client.exe -port 8080
```

Then open <http://localhost:8080>, click the canvas, and move your dot with `WASD`.
Open more tabs (or run more clients) to see multiple players share the same world.

### Docker 
The server can be containerized like so

```powershell
cd server
docker build -t noah_game_server:[tag] .
docker run -d -p 34254:34254/udp noah_game_server:latest
```


---

## Roadmap

The project is built in phases. Each phase has reading attached so the concepts are
learned alongside the code. **Currently finishing Phase 2.**

### Phase 1 — Local UDP Server ✅
*No Kubernetes yet.*
1. A working visual demonstration: dots moving on a canvas driven by the server
2. A Rust UDP server with a stable fixed-rate tick loop running locally
3. Multiple clients can connect and their positions are broadcast every tick
4. A Go test client that connects, sends inputs, and renders received state

### Phase 2 — Authoritative Server ⬅ *almost done*
**Reading:** Building a Game Network Protocol (Glenn Fiedler) · Source Multiplayer
Networking (Valve wiki) · Latency Compensating Methods (Bernier, Valve)
5. Clients send inputs only — the server owns all state
6. Packets are sequenced and selectively reliable

### Phase 3 — Single Pod on Kubernetes
**Reading:** Docker Getting Started · Kubernetes the Hard Way (Kelsey Hightower) ·
Kubernetes Concepts (Pods, Services, Deployments, ConfigMaps) · Agones Architecture
Overview · Agones Quickstart
7. The server runs in a single Kubernetes pod and is reachable over UDP
8. Kubernetes knows when the pod is healthy and when it is not
9. The pod moves through the Agones GameServer lifecycle correctly

### Phase 4 — Load Testing and Baselines
**Reading:** Prometheus Getting Started · USE Method (Brendan Gregg)
10. A bot swarm generates realistic, reproducible load against the server
11. Metrics are visible for tick rate stability, memory, and bandwidth per client
12. Baselines are recorded — every future optimization is measured against these

### Phase 5 — Netcode Optimizations
*Apply one at a time. Measure before and after each.*
**Reading:** Tokio Tutorial · Async Rust Book · Snapshot Compression (Gaffer On
Games) · State Synchronization (Gaffer On Games) · Overwatch Gameplay Architecture
and Netcode (GDC 2017, Timothy Ford) · Valve Lag Compensation Wiki
13. The server can rewind state to compensate for lag *(moved here from Phase 2)*
14. Clients predict their own movement without waiting for the server
15. Remote entity positions are smooth between server snapshots
16. Only state that changed is sent each tick
17. Clients do not receive updates about entities outside their relevant area
18. The server handles significantly more clients than Phase 4 baseline at the same tick rate

### Phase 6 — Fleet and Matchmaking
19. A pool of warm server pods is maintained and scales with demand
20. A Go service routes players to an available pod and returns a connection address
21. The bot swarm can trigger matchmaking end-to-end and the fleet responds correctly

### Phase 7 — Production Hardening
22. Active sessions survive rolling deployments and pod evictions
23. Players connect to the nearest geographic cluster
24. Final bot swarm run is benchmarked against Phase 4 baselines

---

## Project layout

```
server/            Rust authoritative UDP server
  src/main.rs        recv loop + 60 Hz sender/tick loop
  Cargo.toml         serde / serde_json
client/            Go client + browser frontend
  main.go            WebSocket ↔ UDP bridge, input loop
  utils.go           shared types + client state
  index.html         canvas renderer + WASD input
  launch_gamers.ps1  build + launch N clients
```
