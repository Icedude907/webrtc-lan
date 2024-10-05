
export enum ConnectionState{
    Connecting, // When initalising or in the event of a connection hiccup.
    Connected,
    Closed_Fin,
    Closed_Drop,
}

// Transport layer wrapper around webrtc.
// Creating this class causes a connection attempt that will resolve in the future.
export class WebRTCConnection{
    private peer: RTCPeerConnection;
    private data: RTCDataChannel;

    constructor(
        cb_connection_state: (state: ConnectionState)=>void,
        cb_recv: (data: ArrayBuffer)=>void,
    ){
        this.peer = new RTCPeerConnection();
        this.data = this.peer.createDataChannel("c2s", {
            // WebRTC is packet based but normally reliable+ordered in transmission
            // You can disable these by flipping these switches, making the connection UDP-like
            // maxRetransmits: 0, ordered: false,
        })
        this.data.binaryType = "arraybuffer";

        this.data.onclose = ()=>this.peer.close()
        this.data.onmessage = ({data/*ArrayBuffer*/})=>cb_recv(data as ArrayBuffer);
        this.peer.onconnectionstatechange = ()=>{
            const state = this.peer.connectionState;
            switch (state) {
                 case "new": case "connecting": case "disconnected":
                    cb_connection_state(ConnectionState.Connecting);
                    break;
                case "connected":
                    cb_connection_state(ConnectionState.Connected);
                    break;
                case "failed":
                    cb_connection_state(ConnectionState.Closed_Drop);
                    break;
                case "closed":
                    cb_connection_state(ConnectionState.Closed_Fin);
                    break;
            }
        };
    }

    // Attempts to connect to the remote. Returns when connected or failed.
    // cb_connection_state() will run as the connection state changes. (TODO: Precice behaviours?)
    public async connect(){
        // Do the connection
        let offer = await this.peer.createOffer();
        this.peer.setLocalDescription(offer);
        let response = await this.exchange_connection_details(offer);
        this.peer.setRemoteDescription(response.description);
        response.candidates.forEach(e => { this.peer.addIceCandidate(e) });
        // Wait for it to be established
        await new Promise<void>((resolve, reject)=>this.data.onopen = ()=>resolve())
    }

    // Dataview is apparently safe https://developer.mozilla.org/en-US/docs/Web/API/RTCDataChannel/send
    public send(data: Blob|ArrayBuffer|DataView){
        this.data.send(data as any);
    }

    private async exchange_connection_details(offer: RTCSessionDescriptionInit):Promise<{
        description: RTCSessionDescriptionInit,
        candidates: RTCIceCandidateInit[]
    }>{
        let response = await fetch("/connect", {
            method: "POST",
            body: JSON.stringify(offer),
            headers: { "Content-type": "application/json; charset=UTF-8" }
        });
        return await response.json();
    }
}