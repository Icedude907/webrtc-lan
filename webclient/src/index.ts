import * as webrtc from "./webrtc"
import * as packet from "./packets"

let buttonpressed = false

// Requires the page to be loaded.
document.addEventListener('DOMContentLoaded', (_) => {
    document.getElementById('inputBox')!.onkeydown = (event)=>{
        if (event.key === 'Enter') {
            event.preventDefault(); // Prevent the default action (form submission)
            submitMessage();
        }
    };
    document.getElementById('inputSubmit')!.onclick = ()=>submitMessage();
    document.getElementById('usernameBox')!.onkeydown = (e)=>{
        if (e.key === "Enter"){
            e.preventDefault();
            submitNameChange();
        }
    };
    document.getElementById('usernameSubmit')!.onclick = ()=>submitNameChange();

    let presser = document.getElementById('roundButton')!;
    presser.addEventListener('mousedown'  , () => buttonpressed = true );
    presser.addEventListener('mouseup'    , () => buttonpressed = false);
    presser.addEventListener('mouseleave' , () => buttonpressed = false); // To handle the case where the mouse leaves the button
    presser.addEventListener('touchstart' , () => buttonpressed = true );
    presser.addEventListener('touchend'   , () => buttonpressed = false);
    presser.addEventListener('touchcancel', () => buttonpressed = false); // To handle touch cancel event
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
    private users: string[];
    private sessionid: Uint8Array|null;
    private periodic_pinger: number|undefined;

    constructor(){
        this.conn = new webrtc.WebRTCConnection(this.on_connection_state_change, this.recv_packet);
        this.username = "";
        this.users = [];
        this.sessionid = null;
        this.periodic_pinger = undefined;
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
        clearInterval(this.periodic_pinger);
    }

    public send_message(message: string){
        this.conn.send(packet.encode_C2S_SendMsg(message));
    }
    public send_name_change(name: string){
        this.conn.send(packet.encode_C2S_SetName(name));
    }
    public on_connection_established = ()=>{
        this.periodic_pinger = setInterval(() => {
            this.conn.send_unreliable(packet.encode_C2S_Buttons(buttonpressed))
        }, 100); // 100ms
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
            this.on_connection_established()
        }else if(pkt.id === packet.PktS2Cid.ReceiveMsg){
            addToLog(pkt.msg);
        }else if(pkt.id === packet.PktS2Cid.SetNameReply){
            this.set_username(pkt.username);
        }else if(pkt.id === packet.PktS2Cid.LobbyInfo){
            // Just makes sure we don't update the display for no reason.
            if( arrayEqual(pkt.users, this.users) == false ){
                this.users = pkt.users;
                setLobbyText(pkt.users);
            }
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

function arrayEqual<T>(a: T[], b: T[]){
    if (a === b) return true;
    if (a == null || b == null) return false;
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; ++i) {
      if (a[i] !== b[i]) return false;
    }
    return true;
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