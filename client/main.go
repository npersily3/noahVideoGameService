package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net"
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

	udp.request_number = requestNumber
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

	return udp
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

		var message InputMessage

		err = json.Unmarshal(raw, &message)

		if err != nil {
			log.Println(err)
			return
		}

		fmt.Printf("%+v\n", message)

		//TODO, now I have the keystrokes in bools in message
		udpMessage := getClientUDPMessage(message)

		fmt.Printf("udP: %+v\n", udpMessage)

		rawMessage, err := json.Marshal(udpMessage)

		fmt.Printf("raw: %+v\n", rawMessage)

		if err != nil {
			log.Println(err)
			return
		}

		_, err = gameServerConn.Write(rawMessage)

		if err != nil {
			log.Println(err)
			return
		}

		//TODO this nil state is how we send messages back to the frontend
		state := StateMessage{}
		out, err := json.Marshal(state)
		if err != nil {
			log.Println(err)
		}

		err = frontendConn.WriteMessage(websocket.TextMessage, out)

		if err != nil {
			log.Println(err)
		}

	}

}

var gameServerConn net.Conn
var err error

func main() {

	//connect to server
	gameServerConn, err = net.Dial("udp", "127.0.0.1:34254")

	if err != nil {
		panic(err)
	}

	defer gameServerConn.Close()

	_, err = gameServerConn.Write([]byte("hello world"))

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

	http.HandleFunc("/ws", handleWebSocket)
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.ServeFile(w, r, "index.html")
	})
	fmt.Println("WebSocket server is running on :8080/ws")
	fmt.Println("http://localhost:8080")
	err = http.ListenAndServe(":8080", nil)
	if err != nil {
		fmt.Println(err)
	}

}
