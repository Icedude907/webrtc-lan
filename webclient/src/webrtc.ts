import { TypedArray } from "three";

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
    private chanr: RTCDataChannel;
    private chanu: RTCDataChannel;

    constructor(
        cb_connection_state: (state: ConnectionState)=>void,
        cb_recv: (data: ArrayBuffer)=>void,
    ){
        this.peer = new RTCPeerConnection();
        this.chanr = this.create_channel(cb_recv, "ro");
        this.chanu = this.create_channel(cb_recv, "uu", { // unreliable, unordered
            maxRetransmits: 0, ordered: false
        });
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

    private create_channel(cb_recv: (data: ArrayBuffer)=>void, label: string, dataChannelDict?: RTCDataChannelInit){
        let channel = this.peer.createDataChannel(label, dataChannelDict);
        channel.binaryType = "arraybuffer";
        channel.onclose = ()=>this.peer.close()
        channel.onmessage = ({data/*ArrayBuffer*/})=>cb_recv(data as ArrayBuffer);
        return channel;
    }

    // Attempts to connect to the remote. Returns when connected or failed.
    public async connect(){
        // Do the connection
        let offer = await this.peer.createOffer();
        this.peer.setLocalDescription(offer);
        let response = await this.exchange_connection_details(offer);
        this.peer.setRemoteDescription(response.description);
        response.candidates.forEach(e => { this.peer.addIceCandidate(e) });
        // Wait for it to be established (TODO: chanu?)
        await new Promise<void>((resolve, reject)=>this.chanr.onopen = ()=>resolve())
    }

    public disconnect(){
        this.chanr.close();
        this.chanu.close();
        this.peer.close();
    }

    // Dataview is apparently safe https://developer.mozilla.org/en-US/docs/Web/API/RTCDataChannel/send
    public send(data: Blob|ArrayBuffer|DataView|TypedArray){
        this.chanr.send(data as any);
    }
    public send_unreliable(data: Blob|ArrayBuffer|DataView|TypedArray){
        this.chanu.send(data as any);
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