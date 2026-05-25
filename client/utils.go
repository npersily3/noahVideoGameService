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
