package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"net/http"

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

	return udp
}

func convertServerMessageToGameState(serverMessage ServerUDPMessage) StateMessage {
	return StateMessage{
		Type:    "state",
		Players: serverMessage.State.Players,
	}
}

func handleWebSocket(w http.ResponseWriter, r *http.Request) {
	//upgrade http connection to websocket connection
	frontendConn, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}

	defer frontendConn.Close()

	//HandleWebsocket message

	for {
		_, raw, err := frontendConn.ReadMessage()

		if err != nil {
			log.Println(err)
			break
		}

		var inputMessage InputMessage
		var serverMessage ServerUDPMessage

		err = json.Unmarshal(raw, &inputMessage)

		if err != nil {
			log.Println(err)
			return
		}

		//fmt.Printf("%+v\n", inputMessage)

		//TODO, now I have the keystrokes in bools in message
		udpMessage := getClientUDPMessage(inputMessage)

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

		buf := make([]byte, 1024)
		n, err := clientState.serverConn.Read(buf)

		if err != nil {
			log.Println(err)
			return
		}

		err = json.Unmarshal(buf[:n], &serverMessage)

		//TODO this nil state is how we send messages back to the frontend
		gameState := convertServerMessageToGameState(serverMessage)

		out, err := json.Marshal(gameState)
		if err != nil {
			log.Println(err)
		}

		err = frontendConn.WriteMessage(websocket.TextMessage, out)

		if err != nil {
			log.Println(err)
		}

	}

}

var err error
var clientState ClientState

func main() {

	clientState.initClientState()

	defer clientState.serverConn.Close()

	if err != nil {
		panic(err)
	}

	//buf := make([]byte, 1024)
	//n, err := conn.Read(buf)
	//
	//if err != nil {
	//	panic(err)
	//}

	//println(string(buf[:n]))

	port := flag.String("port", "8080", "HTTP port for this client to serve the game on")
	flag.Parse()

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
