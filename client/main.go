package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"math/rand"
	"net"
	"net/http"
	"time"

	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
}

var requestNumber uint32 = 0

func getClientUDPMessage(message InputMessage) ClientUDPMessage {
	udp := ClientUDPMessage{}

	udp.Request_number = requestNumber
	requestNumber++

	var bitmap byte
	bitmap = 0

	if message.Keys.W {
		bitmap |= 1
	}
	if message.Keys.A {
		bitmap |= 2
	}
	if message.Keys.S {
		bitmap |= 4
	}
	if message.Keys.D {
		bitmap |= 8
	}

	udp.UserID = clientState.id
	udp.User_input = bitmap
	udp.LeftClick, udp.MouseX, udp.MouseY = message.LeftClick, message.MousePosition.X, message.MousePosition.Y

	return udp
}

func convertServerMessageToGameState(serverMessage ServerUDPMessage) StateMessage {
	return StateMessage{
		Type:    "state",
		Players: serverMessage.State.Players,
	}
}

// in HZ
const INPUT_FREQUENCY = 60

// this function sends stuff to the server every tick, if there is no tick, it reads the keys
func sendToServerLoop() {

	ticker := time.NewTicker(time.Second / INPUT_FREQUENCY)

	defer ticker.Stop()

	var lastInputMessage InputMessage
	for {

		// non-blocking wait
		select {

		//if there is a new button press, buffer it
		case inputKeys := <-clientState.inputChannel:
			lastInputMessage = inputKeys

		// if the time is up, send it to the server
		case <-ticker.C:
			udpMessage := getClientUDPMessage(lastInputMessage)

			outgoing, err := json.Marshal(udpMessage)

			if err != nil {
				log.Println(err)
				return
			}

			_, err = clientState.serverConn.Write(outgoing)

			if err != nil {
				log.Println(err)
				return
			}
		}
	}
}

func handleWebSocket(w http.ResponseWriter, r *http.Request) {
	//upgrade http connection to websocket connection
	clientState.frontendConn, err = upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}
	defer clientState.frontendConn.Close()

	go updateFrontend()
	go sendToServerLoop()

	for {
		_, raw, err := clientState.frontendConn.ReadMessage()

		if err != nil {
			log.Println(err)
			break
		}

		var inputMessage InputMessage

		err = json.Unmarshal(raw, &inputMessage)

		if err != nil {
			log.Println(err)
			return
		}

		clientState.inputChannel <- inputMessage
	}
}

var err error
var clientState ClientState
var address *string

type ClientState struct {
	id           uint64
	player       PlayerState
	serverConn   net.Conn
	upgrader     websocket.Upgrader
	frontendConn *websocket.Conn
	inputChannel chan InputMessage
}

func (c *ClientState) initClientState() {
	c.player = PlayerState{
		X: 0,
		Y: 0,
	}
	c.id = rand.Uint64()

	c.serverConn, err = net.Dial("udp", *address)

	if err != nil {
		panic(err)
	}

	c.inputChannel = make(chan InputMessage, 16)
}

func main() {

	// Parse flags before anything uses them: initClientState dials *address, and
	// the HTTP server below uses *port. Parsing afterwards left both stuck on
	// their defaults, so -k8s (e.g. the clustered 7165 hostPort) was ignored.
	address = flag.String("k8s", "127.0.0.1:34254", "k8s server address")
	port := flag.String("port", "8080", "HTTP port for this client to serve the game on")
	flag.Parse()

	clientState.initClientState()

	defer clientState.serverConn.Close()

	if err != nil {
		panic(err)
	}

	addr := ":" + *port

	http.HandleFunc("/ws", handleWebSocket)
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.ServeFile(w, r, "index.html")
	})
	fmt.Printf("WebSocket server is running on %s/ws\n", addr)
	fmt.Printf("http://localhost:%s\n", *port)
	err = http.ListenAndServe(addr, nil)
	if err != nil {
		fmt.Println(err)
	}

}

func updateFrontend() {
	for {

		var serverMessage ServerUDPMessage

		buf := make([]byte, 1024)
		n, err := clientState.serverConn.Read(buf)

		if err != nil {
			log.Println(err)
			return
		}

		err = json.Unmarshal(buf[:n], &serverMessage)

		//todo, once we have delta compression have a global state that is updated etc
		gameState := convertServerMessageToGameState(serverMessage)

		out, err := json.Marshal(gameState)
		if err != nil {
			log.Println(err)
		}

		err = clientState.frontendConn.WriteMessage(websocket.TextMessage, out)

		if err != nil {
			log.Println(err)
		}
	}
}
