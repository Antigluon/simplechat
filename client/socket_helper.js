send_messages = (n) => {
    let s = new WebSocket("ws://127.0.0.1:1234/connect")
    s.onopen = (e) => {
        s.send("rate_test")
        for (i=0;i<n;i++) {
            s.send("message #" + i)
        }
        s.close()
    }
}

make_connections = (n) => {
    let connections = []
    for (i = 0; i<n; i++) {
        connections[i] = new WebSocket("ws://127.0.0.1:1234/connect")
    }
    return connections;
}
