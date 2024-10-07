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
    document.getElementById('usernameBox')!.addEventListener('keypress', (e)=>{
        if (e.key === "Enter"){
            e.preventDefault();
            submitNameChange();
        }
    })
})
// Destroys the webrtc connection, rather than having to wait for a timeout event on the server side.
window.addEventListener('beforeunload', (_)=>{
    sess?.shutdown();
})

let sess: Session | undefined;

async function main(){
    sess = new Session();
    await sess.connect();
}

class Session{
    private conn: webrtc.WebRTCConnection;
    private username: string;
    private sessionid: Uint8Array|null;

    constructor(){
        this.conn = new webrtc.WebRTCConnection(this.on_connection_state_change, this.recv_packet);
        this.username = "";
        this.sessionid = null;
    }

    public async connect(){
        await this.conn.connect()
        // send Hello
        this.conn.send(packet.encode_C2S_Hello(null));
    }
    // Destroy the connection and the session on page close
    public shutdown(){
        this.conn.send(packet.encode_C2S_Goodbye());
        this.conn.disconnect();
        sess = undefined;
    }

    public send_message(message: string){
        this.conn.send(packet.encode_C2S_SendMsg(message));
    }
    public send_name_change(name: string){
        this.conn.send(packet.encode_C2S_SetName(name));
    }

    // Arrow function inherits this, but regular function does not. WHAT
    private recv_packet = (data: ArrayBuffer)=>{
        let _pkt = packet.decode_packet(data);
        console.log(`Received packet: ${JSON.stringify(_pkt)}`);
        if(typeof _pkt === "string"){
            console.log(`packet error: ${_pkt}`);
            return;
        }
        let pkt = _pkt as (packet.PacketS2C & any);
        if(pkt.id === packet.PktS2Cid.HelloReply){
            this.sessionid = pkt.sid;
            this.set_username(pkt.username)
        }else if(pkt.id === packet.PktS2Cid.ReceiveMsg){
            addToLog(pkt.msg);
        }else if(pkt.id === packet.PktS2Cid.SetNameReply){
            this.set_username(pkt.username);
        }else if(pkt.id === packet.PktS2Cid.LobbyInfo){
            setLobbyText(pkt.users);
        }
    }

    private on_connection_state_change = (state: webrtc.ConnectionState)=>{
        setConnectionStatusText(webrtc.ConnectionState[state])
    }
    private set_username(username: string){
        this.username = username;
        setUsernameText(username);
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
function setLobbyText(users: string[]){
    const table = document.getElementById("lobby");
    if(table == null) return;
    const tablebody = table.querySelector("tbody");
    tablebody?.remove();
    let rows: Node[] = [];
    users.forEach(s=>{
        const row = document.createElement('tr')
        const cell = document.createElement('td');
        cell.textContent = s;
        row.appendChild(cell);
        rows.push(row);
    });
    const body = document.createElement("tbody");
    body.append(...rows);
    table.append(body);
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
function submitNameChange(){
    const inputBox = document.getElementById('usernameBox')! as HTMLInputElement;
    sess?.send_name_change(inputBox.value);
    inputBox.value = "...pending";
}

// The act of connecting to the server actually doesn't require the page to be finished loading.
main();