package main

import (
	"flag"
	"fmt"
	"log"
	"math/rand"
	"net"
	"time"
)

type botInfo struct {
	userID        uint64
	requestNumber uint32
	perspective   uint32
}

func (info *botInfo) getUDPMessage() ClientUDPMessage {

	var input uint8 = 0
	click := true

	//dumb random wasd
	if rand.Int()%2 == 0 {
		input |= 1
	}
	if rand.Int()%2 == 0 {
		input |= 2
	}
	if rand.Int()%2 == 0 {
		input |= 4
	}
	if rand.Int()%2 == 0 {
		input |= 8
	}

	if rand.Int()%2 == 0 {
		click = false
	}

	message := ClientUDPMessage{
		UserID:             info.userID,
		Request_number:     info.requestNumber,
		User_input:         input,
		LeftClick:          click,
		MouseX:             rand.Int31n(1200),
		MouseY:             rand.Int31n(800),
		Client_perspective: info.perspective,
	}

	info.requestNumber++
	info.perspective++

	return message

}

func serverBot(addr *string) {
	conn, err := net.Dial("udp", *addr)

	if err != nil {
		log.Fatal(err)
	}

	defer conn.Close()

	info := botInfo{
		userID:        rand.Uint64(),
		perspective:   0,
		requestNumber: 0,
	}

	for {
		message := info.getUDPMessage()

		outgoing := message.Serialize()

		_, err = conn.Write(outgoing)

		if err != nil {
			log.Println(err)
			return
		}
		// [3,8] millisecond interval
		time.Sleep(time.Duration(rand.Intn((5))+3) * time.Millisecond)
	}
}
func clientBot() {

}
func main() {
	botAddress := flag.String("k8s", "127.0.0.1:34254", "k8s server address")
	numBots := *flag.Int("bots", 300, "number of bots")
	flag.Parse()

	for i := 0; i < numBots; i++ {
		go serverBot(botAddress)
		fmt.Printf("Bot %d started\n", i)
	}

	time.Sleep(time.Duration(20 * time.Second))
}
