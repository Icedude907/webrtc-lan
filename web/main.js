let comm = undefined;

// Connect to the WebRTC server
document.addEventListener('DOMContentLoaded', (event) => {
    connect();
})
async function connect(){
    const local = new RTCPeerConnection()
    comm = local.createDataChannel("c2s", {
        // WebRTC is packet based but normally reliable+ordered in transmission
        // You can disable these by flipping these switches, making the connection UDP-like
        // maxRetransmits: 0, ordered: false,
    })
    setupConectionHandlers(local, comm);
    // Do the connection
    let offer = await local.createOffer();
    local.setLocalDescription(offer);
    let response = await exchange_connection_details(offer);
    local.setRemoteDescription(response.description);
    response.candidates.forEach(e => { local.addIceCandidate(e) });
}
// Detail exchange
async function exchange_connection_details(offer){
    let response = await fetch("/connect", {
        method: "POST",
        body: JSON.stringify(offer),
        headers: { "Content-type": "application/json; charset=UTF-8" }
    });
    return await response.json();
}
function setupConectionHandlers(peer, channel){
    // State handlers
    channel.onopen  = () => setConnectionStatus("Chan Connected");
    channel.onclose = () => {
        setConnectionStatus("Chan Disconnected");
        peer.close(); // If the channel is unexpectedly closed our connection is finished.
    }

    peer.onconnectionstatechange = (e) => {
        const state = peer.connectionState;

        switch (state) {
             case "new": case "connecting":
                setConnectionStatus("Connecting...");
                break;
            case "connected":
                setConnectionStatus("Connection established");
                break;
            case "disconnected":
                setConnectionStatus("Connection interrupted. Attempting to re-establish...");
                break;
            case "failed":
                setConnectionStatus("Connection finished (failed)");
                break;
            case "closed":
                setConnectionStatus("Connection finished (closed)");
                break;
        }
    }
    // Data handler
    channel.onmessage = ({data}) => {
        let msg = new TextDecoder("utf-8").decode(data);
        addToLog(msg);
    }
}

function setConnectionStatus(status){
    console.log(`Status -> ${status}`);
    document.getElementById("connectionStatus").innerText = status;
}

function addToLog(msg){
    const displayBox = document.getElementById('displayBox');
    displayBox.value += msg + '\n';
}
// Send messages
function submitMessage() {
    const inputBox = document.getElementById('inputBox');
    comm.send(new TextEncoder("utf-8").encode(inputBox.value));
    inputBox.value = '';
}
document.getElementById('inputBox').addEventListener('keypress', function(event) {
    if (event.key === 'Enter') {
        event.preventDefault(); // Prevent the default action (form submission)
        submitMessage();
    }
});