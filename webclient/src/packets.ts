// Encoding
// ---------------
enum PktC2S{
    Hello = 0,
    SendMsg = 1,
    SetName = 2,
}

export function encode_C2S_Hello(cached_sid: Uint8Array | null): ArrayBuffer{
    let length = 1;
    if(cached_sid !== null){ length += 8; }
    let buffer = new ArrayBuffer(length);

    let view = new Uint8Array(buffer);
    view[0] = PktC2S.Hello;
    if(cached_sid !== null){
        view.set(cached_sid, 1);
    }

    return buffer;
}

export function encode_C2S_SendMsg(message: string): ArrayBuffer{
    let length = 1;
    let msg = new TextEncoder().encode(message);
    length += msg.length;
    let buffer = new ArrayBuffer(length);
    let view = new Uint8Array(buffer);
    view[0] = PktC2S.SendMsg;
    view.set(msg, 1);
    return buffer;
}

export function encode_C2S_SetName(name: string): ArrayBuffer{
    let length = 1;
    let msg = new TextEncoder().encode(name);
    length += msg.length;
    let buffer = new ArrayBuffer(length);
    let view = new Uint8Array(buffer);
    view[0] = PktC2S.SetName;
    view.set(msg, 1);
    return buffer;
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
        this.ofs += count;
        return new Uint8Array(this.view.buffer, this.ofs, count);
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
    public get_str(): string {
        let length = this.get_uvarint();
        const stringBytes = new Uint8Array(this.view.buffer, this.ofs, length);
        this.ofs += length;
        return new TextDecoder('utf-8').decode(stringBytes);
    }
    public get_arr<T>(reader: (decoder: PktDecoder)=>T): T[]{
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
        username: d.get_str(),
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
        users: d.get_arr((d)=>d.get_str()),
    }
}

// "Lookup table" that decodes incoming packets into legible types.
const PktDecodeLookup: { [id in PktS2Cid]: DecoderFunction<any>} = {
    [PktS2Cid.HelloReply]: decode_S2C_HelloReply,
    [PktS2Cid.LobbyInfo]: decode_S2C_LobbyInfo,
    [PktS2Cid.ReceiveMsg]: decode_S2C_ReceiveMsg,
    [PktS2Cid.SetNameReply]: decode_S2C_SetNameReply,
};