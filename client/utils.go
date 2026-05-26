package main

// matches what the browser sends
type InputMessage struct {
	Type string `json:"type"`
	Keys struct {
		W bool `json:"w"`
		A bool `json:"a"`
		S bool `json:"s"`
		D bool `json:"d"`
	} `json:"keys"`
}

// matches what the browser expects
type StateMessage struct {
	Type    string                 `json:"type"`
	Players map[string]PlayerState `json:"players"`
}
type PlayerState struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

type ClientUDPMessage struct {
	//keep track of which input to send
	request_number uint32 `json:"request_number"`
	//which input was sent, this is a bitmap so if I press w and s the map will look like 1010
	user_input uint8 `json:"inputBitmap"`
}

type ServerUDPMessage struct {
	request_number uint32       `json:"request_number"`
	State         StateMessage `json:"state"`
}
