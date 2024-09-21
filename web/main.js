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
        // It might be worth doing this in the future.
        // maxRetransmits: 0, ordered: false,
    })
    // Handlers
    comm.onopen = () => {
        setConnectionStatus("Channel Connected");
    }
    comm.onclose = () => {
        setConnectionStatus("Channel Disconnected");
    }
    comm.onmessage = ({data}) => {
        let msg = new TextDecoder("utf-8").decode(data);
        addToLog(msg);
    }
    local.onconnectionstatechange = (e) => {
        const state = this.iceConnectionState;
        switch (state) {
            case "new": case "connecting":
                setConnectionStatus("Connecting…");
                break;
            case "connected":
                setConnectionStatus("Connection Online");
                break;
            case "disconnected":
                setConnectionStatus("Disconnecting…");
                break;
            case "closed":
                setConnectionStatus("Connection Offline");
                break;
            case "failed":
                setConnectionStatus("Connection Error");
                break;
            case undefined:
                break;
            default:
                setConnectionStatus(`Unknown: ${state}`);
        }
    }
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