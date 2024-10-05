import * as webrtc from "./webrtc"
import * as packet from "./packets"

// Requires the page to be loaded.
document.addEventListener('DOMContentLoaded', (_) => {
    document.getElementById('inputBox')!.addEventListener('keypress', function(event) {
        if (event.key === 'Enter') {
            event.preventDefault(); // Prevent the default action (form submission)
            submitMessage();
        }
    });
})

let sess: Session | undefined;

async function main(){
    sess = new Session();
    await sess.connect();
}

class Session{
    private conn: webrtc.WebRTCConnection;
    private username: string;

    constructor(){
        this.conn = new webrtc.WebRTCConnection(this.on_connection_state_change, this.recv_packet);
        this.username = "<connecting>";
    }

    public async connect(){
        await this.conn.connect()
    }

    public send_message(message: string){
        this.conn.send(packet.encode_C2S_SendMsg(message));
    }

    private recv_packet(data: ArrayBuffer){
        let _pkt = packet.decode_packet(data);
        console.log(`Received packet: ${_pkt}`);
        if(typeof _pkt === "string"){
            console.log(`packet error: ${_pkt}`);
            return;
        }
        let pkt = _pkt as (packet.PacketS2C & any);
        if(pkt.id === packet.PktS2Cid.HelloReply){
            console.log(`Hello reply: ${pkt}`);
        }else if(pkt.id === packet.PktS2Cid.ReceiveMsg){
            addToLog(pkt.msg);
        }else if(pkt.id === packet.PktS2Cid.SetNameReply){
            setUsernameText(pkt.username);
        }else if(pkt.id === packet.PktS2Cid.LobbyInfo){
            console.log(`Lobby info: ${pkt}`);
        }
    }

    private on_connection_state_change(state: webrtc.ConnectionState){
        setConnectionStatusText(webrtc.ConnectionState[state])
    }
}

function setConnectionStatusText(status: string){
    console.log(`Status -> ${status}`);
    document.getElementById("connectionStatus")!.innerText = status;
}
function setUsernameText(username: string){
    let box = document.getElementById("usernameBox");
    if(box == null) return;
    (box as HTMLInputElement).value = username;
}

function addToLog(msg: string){
    const displayBox = document.getElementById('displayBox') as HTMLInputElement;
    displayBox.value += msg + '\n';
}

// Send messages
function submitMessage() {
    const inputBox = document.getElementById('inputBox')! as HTMLInputElement;
    sess?.send_message(inputBox.value);
    inputBox.value = '';
}

// The act of connecting to the server actually doesn't require the page to be finished loading.
main();