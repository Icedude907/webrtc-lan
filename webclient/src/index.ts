let comm: RTCDataChannel;

// Connect to the WebRTC server
document.addEventListener('DOMContentLoaded', (_) => {
    connect();
})
async function connect(){
    const local = new RTCPeerConnection()
    comm = local.createDataChannel("c2s", {
        // WebRTC is packet based but normally reliable+ordered in transmission
        // You can disable these by flipping these switches, making the connection UDP-like
        // maxRetransmits: 0, ordered: false,
    })
    comm.binaryType = "arraybuffer";
    setupConectionHandlers(local, comm);
    // Do the connection
    let offer = await local.createOffer();
    local.setLocalDescription(offer);
    let response: {
        description: RTCSessionDescriptionInit,
        candidates: RTCIceCandidateInit[]
    } = await exchange_connection_details(offer);
    local.setRemoteDescription(response.description);
    response.candidates.forEach(e => { local.addIceCandidate(e) });
}
// Detail exchange
async function exchange_connection_details(offer: RTCSessionDescriptionInit){
    let response = await fetch("/connect", {
        method: "POST",
        body: JSON.stringify(offer),
        headers: { "Content-type": "application/json; charset=UTF-8" }
    });
    return await response.json();
}
function setupConectionHandlers(peer: RTCPeerConnection, channel: RTCDataChannel){
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
    channel.onmessage = ({data /*ArrayBuffer*/}) => {
        let msg = new TextDecoder("utf-8").decode(data);
        addToLog(msg);
    }
}

function setConnectionStatus(status: string){
    console.log(`Status -> ${status}`);
    document.getElementById("connectionStatus")!.innerText = status;
}

function addToLog(msg: string){
    const displayBox = document.getElementById('displayBox') as HTMLInputElement;
    displayBox.value += msg + '\n';
}
// Send messages
function submitMessage() {
    const inputBox = document.getElementById('inputBox') as HTMLInputElement;
    comm.send(new TextEncoder().encode(inputBox.value));
    inputBox.value = '';
}
document.getElementById('inputBox')!.addEventListener('keypress', function(event) {
    if (event.key === 'Enter') {
        event.preventDefault(); // Prevent the default action (form submission)
        submitMessage();
    }
});