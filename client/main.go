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

func handleWebSocket(w http.ResponseWriter, r *http.Request) {
	//upgrade http connection to websocket connection
	conn, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		log.Println(err)
		return
	}

	defer conn.Close()

	//HandleWebsocket message

	for {
		_, raw, err := conn.ReadMessage()

		if err != nil {
			log.Println(err)
			break
		}

		var message InputMessage

		err = json.Unmarshal(raw, &message)

		if err != nil {
			log.Println(err)
		}

		fmt.Printf("%+v\n", message)

		//TODO, now I have the keystrokes in bools in message

		//TODO this nil state is how we send messages back to the frontend
		state := StateMessage{}
		out, err := json.Marshal(state)
		if err != nil {
			log.Println(err)
		}

		err = conn.WriteMessage(websocket.TextMessage, out)

		if err != nil {
			log.Println(err)
		}

	}

}

func main() {

	//connect to server
	conn, err := net.Dial("udp", "127.0.0.1:34254")

	if err != nil {
		panic(err)
	}

	defer conn.Close()

	_, err = conn.Write([]byte("hello world"))

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
