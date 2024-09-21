async function exchange_connection_details(o){
    return fetch("/connect", {
        method: "POST",
        body: JSON.stringify(o),
        headers: { "Content-type": "application/json; charset=UTF-8" }
    }).then(r=>r.json())
}

const local = new RTCPeerConnection()
    const channel_c2s = local.createDataChannel("c2s", {
        // WebRTC is packet based but normally reliable+ordered in transmission
        // You can disable these by flipping these switches, making the connection UDP-like
        // It might be worth doing this in the future.

        // maxRetransmits: 0,
        // ordered: false,
    })
    channel_c2s.onopen = () => console.log("Channel Connected")
    channel_c2s.onclose = () => console.log("Channel Disconnected")
    channel_c2s.onmessage = ({data}) => console.log(`RECV: ${data}`)
    const s2c = local.createDataChannel("s2c", {});
let offer = local.createOffer()
offer.then(o=>{
    local.setLocalDescription(o)
    exchange_connection_details(o).then(r=>{
        console.log(r)
        local.setRemoteDescription(r.description)
        r.candidates.forEach(e => { local.addIceCandidate(e) });
        console.log("Setup done");
    })
})

channel_c2s.send("Hello from client!");