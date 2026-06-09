package main

import (
	"math/rand"
	"net"

	"github.com/gorilla/websocket"
)

// matches what the browser sends
type InputMessage struct {
	Type string `json:"type"`
	Keys struct {
		W bool `json:"w"`
		A bool `json:"a"`
		S bool `json:"s"`
		D bool `json:"d"`
	} `json:"keys"`
	LeftClick     bool          `json:"left_click"`
	MousePosition MousePosition `json:"mouse_position"`
}

type MousePosition struct {
	X int32 `json:"x"`
	Y int32 `json:"y"`
}

// matches what the browser expects
type StateMessage struct {
	Type    string                 `json:"type"`
	Players map[string]PlayerState `json:"players"`
}
type PlayerState struct {
	X      float64 `json:"x"`
	Y      float64 `json:"y"`
	Health int32   `json:"health"`
	Angle  float64 `json:"angle"`
}

type ClientUDPMessage struct {
	UserID uint64 `json:"user_id"`
	//keep track of which input to send
	Request_number uint32 `json:"request_number"`
	//which input was sent, this is a bitmap so if I press w and s the map will look like 1010
	User_input uint8 `json:"input_bitmap"`
	LeftClick  bool  `json:"left_click"`
	MouseX     int32 `json:"mouse_x"`
	MouseY     int32 `json:"mouse_y"`
	// the world in which the client is moving in
	client_perspective uint32 `json:"client_perspective"`
}

type ServerUDPMessage struct {
	Request_number uint32       `json:"request_number"`
	State          StateMessage `json:"state"`
}

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
