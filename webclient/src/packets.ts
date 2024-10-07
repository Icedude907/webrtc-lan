// Encoding
// ---------------
enum PktC2Sid{
    Hello = 0,
    SendMsg = 1,
    SetName = 2,
}

// NOTE: Resiable ArrayBuffer is not avaliable enough to warrant using it in this code.
// Nor is there an appopriate substitution for it at this time in the transpiler stages.
// Safari iOS <= 16.3 is the big compatibility breaker. https://caniuse.com/mdn-javascript_builtins_arraybuffer_resize
class PacketEncoder{
    private buf = new ArrayBuffer(64);
    private idx = 0;

    constructor(){}
    // Makes sure adding `amount` bytes to the buffer won't overflow.
    private reserve_extra(amount: number){
        let curcap = this.buf.byteLength;
        let next = amount + this.idx;
        if(next > curcap){ // Copy into new buffer
            let newcap = next + Math.min(1024, next); // Double or add 1kb
            let newbuf = new ArrayBuffer(newcap);
            new Uint8Array(newbuf).set(new Uint8Array(this.buf));
            this.buf = newbuf;
        }
    }
    // Creates a view starting from the current position in the writer
    private view(){
        return new Uint8Array(this.buf, this.idx)
    }
    // Get a view over the packet that's the correct size for sending
    public finish(){
        return new DataView(this.buf, 0, this.idx)
    }

    // Wraps
    public append_u8(n: number){
        this.reserve_extra(1);
        this.view()[0] = n;
        this.idx += 1;
    }
    public append_bytes(bytes: Uint8Array){
        this.reserve_extra(bytes.length);
        this.view().set(bytes);
        this.idx += bytes.length;
    }
    // Truncates larger numbers than u28
    public append_uvarint(num: number){
        this.reserve_extra(4);
        let view = this.view();
        let n = 0;
        for(let i = 0; i < 4; i++){
            view[i] = num & 0x7f;
            num >>= 7;
            if(num !== 0){ view[i] |= 0x80 }
            this.idx += 1;
            if(num === 0){ break }
        }
    }
    public append_str(dat: string){
        let msg = new TextEncoder().encode(dat);
        this.append_uvarint(msg.length);
        this.reserve_extra(msg.length);
        this.view().set(msg);
        this.idx += msg.length;
    }
    public append_exhaustive_str(dat: string){
        let msg = new TextEncoder().encode(dat);
        this.reserve_extra(msg.byteLength);
        this.view().set(msg);
        this.idx += msg.byteLength;
    }
}

export function encode_C2S_Hello(cached_sid: Uint8Array | null){
    let enc = new PacketEncoder();
    enc.append_u8(PktC2Sid.Hello);
    if(cached_sid !== null){
        enc.append_bytes(cached_sid);
    }
    return enc.finish();
}

export function encode_C2S_SendMsg(message: string){
    let enc = new PacketEncoder();
    enc.append_u8(PktC2Sid.SendMsg);
    enc.append_exhaustive_str(message);
    return enc.finish();
}

export function encode_C2S_SetName(name: string){
    let enc = new PacketEncoder();
    enc.append_u8(PktC2Sid.SetName);
    enc.append_exhaustive_str(name);
    return enc.finish();
}


// Decoding
// ---------------

export enum PktS2Cid{
    HelloReply = 0,
    ReceiveMsg = 1,
    SetNameReply = 2,
    LobbyInfo = 3,
}

export interface PacketS2C{
    id: PktS2Cid
}

type PktS2C_HelloReply = PktS2C_SetNameReply & {
    sid: Uint8Array,
    username: string,
}
type PktS2C_ReceiveMsg = {
    msg: string,
}
type PktS2C_SetNameReply = {
    username: string,
}
type PktS2C_LobbyInfo = {
    users: string[],
}

export enum ParseError{
    Unimplemented,
    UnknownPacket
}

class PktDecoder{
    private view: DataView;
    private ofs: number;

    constructor(buffer: ArrayBuffer){
        this.view = new DataView(buffer)
        this.ofs = 0;
    }

    public get_u8(): number {
        const value = this.view.getUint8(this.ofs);
        this.ofs += 1;
        return value;
    }
    public get_bytes(count: number): Uint8Array{
        let value = new Uint8Array(this.view.buffer, this.ofs, count);
        this.ofs += count;
        return value;
    }

    public get_uvarint(): number {
        let shift = 0;
        let val = 0;
        let n = 0;
        while(true){
            n++;
            let byte = this.get_u8();
            val += (byte & 0x7f) << shift;
            shift += 7;
            if((byte & 0x80) === 0 || n === 4){ break; }
        }
        return val;
    }
    public get_str_len(len: number): string{
        const stringBytes = new Uint8Array(this.view.buffer, this.ofs, len);
        this.ofs += len;
        return new TextDecoder('utf-8').decode(stringBytes);
    }
    public get_str(){
        let length = this.get_uvarint();
        return this.get_str_len(length);
    }
    public get_str_exhaustive(){
        let length = this.view.byteLength - this.offset;
        return this.get_str_len(length);
    }
    // Calls the provided function N times as determined by the next uvarint length
    public get_arr<T>(reader: (d: PktDecoder)=>T): T[]{
        let length = this.get_uvarint();
        return Array.from({length: length}, ()=>reader(this));
    }

    public get_sessionid(): Uint8Array{
        return this.get_bytes(8);
    }

    public get offset() {
        return this.ofs;
    }
}

type DecoderResult<T> = T | ParseError;
type DecoderFunction<T = void> = (decoder: PktDecoder) => DecoderResult<T>

export function decode_packet(buffer: ArrayBuffer): PacketS2C | ParseError{
    let decoder = new PktDecoder(buffer);
    let id = decoder.get_u8();
    let decode_function = PktDecodeLookup[id as PktS2Cid];
    if(!decode_function){
        return ParseError.UnknownPacket
    }
    let result = decode_function(decoder);
    if(result in ParseError){
        return result;
    }
    return {
        id: id,
        ...result
    }
}

let decode_unimplemented: DecoderFunction = (_)=>ParseError.Unimplemented;

let decode_S2C_HelloReply: DecoderFunction<PktS2C_HelloReply> = (d)=>{
    return {
        sid: d.get_sessionid(),
        username: d.get_str_exhaustive(),
    };
}
let decode_S2C_ReceiveMsg: DecoderFunction<PktS2C_ReceiveMsg> = (d)=>{
    return {
        msg: d.get_str(),
    }
}
let decode_S2C_SetNameReply: DecoderFunction<PktS2C_SetNameReply> = (d)=>{
    return {
        username: d.get_str(),
    }
}
let decode_S2C_LobbyInfo: DecoderFunction<PktS2C_LobbyInfo> = (d)=>{
    return {
        users: d.get_arr((d)=>{
            // console.log(`${JSON.stringify(d)}`);
            return d.get_str();
        }),
    }
}

// "Lookup table" that decodes incoming packets into legible types.
const PktDecodeLookup: { [id in PktS2Cid]: DecoderFunction<any>} = {
    [PktS2Cid.HelloReply]: decode_S2C_HelloReply,
    [PktS2Cid.ReceiveMsg]: decode_S2C_ReceiveMsg,
    [PktS2Cid.SetNameReply]: decode_S2C_SetNameReply,
    [PktS2Cid.LobbyInfo]: decode_S2C_LobbyInfo,
};