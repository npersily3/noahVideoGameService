package main

import (
	"net"
)

func main() {
	conn, err := net.Dial("udp", "127.0.0.1:34254")

	if err != nil {
		panic(err)
	}

	defer conn.Close()

	_, err = conn.Write([]byte("hello world"))

	if err != nil {
		panic(err)
	}

	buf := make([]byte, 1024)
	n, err := conn.Read(buf)

	if err != nil {
		panic(err)
	}

	println(string(buf[:n]))
}
