#![allow(non_camel_case_types)]

use core::str;
use std::io::{Cursor, Write};

use derive_more::{derive::Debug, From};
use derive_new::new;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::usersession::SessionId;

// TODO: Improve networking ergonomics
// pub poison is big sad
// lots of boilerplates and results
// and I still need to derive MORE

// Network representation of packets
#[derive(FromPrimitive)]
enum PktC2Sid{
    Hello = 0,
    SendMsg = 1,
    SetName = 2,
}

#[repr(u8)]
enum PktS2Cid{
    HelloReply = 0,
    ReceiveMsg = 1,
    SetNameReply = 2,
    LobbyInfo = 3,
}

// In memory representation of a packet
#[derive(From, Debug)]
pub enum PktC2S{
    Hello(PktC2S_Hello),
    SendMsg(PktC2S_SendMsg),
    SetName(PktC2S_SetName),
}
#[derive(Debug)] pub struct PktC2S_Hello{pub sid: Option<SessionId>}
#[derive(Debug)] pub struct PktC2S_SendMsg{pub msg: String}
#[derive(Debug)] pub struct PktC2S_SetName{pub name: String}

#[derive(From)]
enum PktS2C{
    HelloReply(PktS2C_HelloReply),
    ReceiveMsg(PktS2C_ReceiveMsg),
    SetNameReply(PktS2C_SetNameReply),
    LobbyInfo(PktS2C_LobbyInfo)
}
#[derive(new)] pub struct PktS2C_HelloReply{sid: SessionId, username: String}
#[derive(new)] pub struct PktS2C_ReceiveMsg{msg: String}
#[derive(new)] pub struct PktS2C_SetNameReply{name: String}
#[derive(new)] pub struct PktS2C_LobbyInfo{users: Vec<String>}

// Encoding and decoding traits
pub trait Encode{
    fn encode(self) -> Vec<u8>;
}
trait Decode{
    fn decode(src: &mut Decoder) -> Result<Self, ()> where Self: Sized;
}

// Helper reader and writer classes
#[derive(new)]
struct Decoder{
    src: Vec<u8>,
    #[new(value = "0")]
    idx: usize,
}
type R<T> = Result<T, ()>;
impl Decoder{
    // Associate
    // use self::SessionId;

    pub fn len(&self)->usize{
        self.src.len()
    }
    // 0 = exhausted
    pub fn rem(&self)->usize{
        self.len().saturating_sub(self.idx)
    }

    // All functions return results: Err signifies some problem.
    // If error, the index is modified so be careful (FIXME).
    pub fn get_u8(&mut self)->R<u8>{
        let Some(ret) = self.src.get(self.idx).copied() else {return Err(())};
        self.idx += 1;
        return Ok(ret);
    }
    pub fn get_bytes_const<const N: usize>(&mut self)->R<[u8; N]>{
        let slice = self.src.get(self.idx..self.idx+N);
        let Some(slice) = slice else{ return Err(()) };
        let mut arr: [u8; N] = [0; N];
        arr.copy_from_slice(slice);
        self.idx += N;
        return Ok(arr);
    }
    pub fn get_uvarint(&mut self)->R<u32>{
        // Fun fact - this code was almost identical to the typescript implementation
        let mut shift = 0;
        let mut val: u32 = 0;
        let mut n = 0;
        loop{
            n += 1;
            let byte = self.get_u8()? as u32;
            val += (byte & 0x7f) << shift;
            shift += 7;
            if byte & 0x80 == 0 || n == 4 { break; }
        }
        return Ok(val);
    }
    pub fn get_str_len(&mut self, len: usize)->R<String>{
        let Some(slice) = self.src.get(self.idx..self.idx+len) else {return Err(())};
        self.idx += len;
        let Ok(str) = str::from_utf8(slice) else {return Err(())};
        return Ok(str.into());
    }
    pub fn get_str(&mut self)->R<String>{
        let len = self.get_uvarint()?;
        return self.get_str_len(len as usize);
    }
    pub fn get_exhaustive_str(&mut self)->String{ // Should be infallible?
        return self.get_str_len(self.rem()).unwrap_or_default();
    }
    pub fn get_arr<F: Fn(&mut Self)->R<T>, T>(&mut self, reader: F)->R<Vec<T>>{
        let len = self.get_uvarint()? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len{
            vec.push(reader(self)?);
        }
        return Ok(vec);
    }
    pub fn get_sessionid(&mut self)->R<SessionId>{
        return self.get_bytes_const::<8>().map(|x| SessionId(u64::from_le_bytes(x)));
    }
}

#[derive(new)]
struct Encoder{
    #[new(default)]
    buf: Cursor<Vec<u8>>,
}
impl Encoder{
    fn with_capacity(cap: usize)->Self{
        Self { buf: Cursor::new(Vec::with_capacity(cap)) }
    }
    fn consume(self) -> Vec<u8>{
        self.buf.into_inner()
    }
    fn append_u8(&mut self, dat: u8){
        self.buf.write(&[dat]);
    }
    fn append_bytes(&mut self, dat: &[u8]){
        self.buf.write(dat);
    }
    // Uvarints are 28 bits wide.
    fn append_uvarint(&mut self, dat: u32){
        let mut dat = dat;
        for _ in 0..4{
            let mut tmp = dat as u8 & 0x7f;
            dat >>= 7;
            if dat != 0 { tmp |= 0x80; }
            self.buf.write(&[tmp]);
            if dat == 0 { break; }
        }
    }
    fn append_str(&mut self, dat: &str){
        self.append_uvarint(dat.len() as u32);
        self.buf.write(dat.as_bytes());
    }
    fn append_exhaustive_str(&mut self, dat: &str){
        self.buf.write(dat.as_bytes());
    }
    fn append_sessionid(&mut self, dat: SessionId){
        self.append_bytes(&dat.0.to_le_bytes());
    }
}

// Implementing encode and decode
impl Decode for PktC2S_Hello{
    fn decode(src: &mut Decoder) -> Result<Self, ()> {
        let sid = src.get_sessionid().ok();
        Ok(Self { sid })
    }
}
impl Decode for PktC2S_SendMsg{
    fn decode(src: &mut Decoder) -> Result<Self, ()> {
        let msg = src.get_exhaustive_str();
        Ok(Self { msg })
    }
}
impl Decode for PktC2S_SetName{
    fn decode(src: &mut Decoder) -> Result<Self, ()> {
        let name = src.get_exhaustive_str();
        Ok(Self { name })
    }
}
impl Encode for PktS2C_HelloReply{
    fn encode(self) -> Vec<u8> {
        let mut enc = Encoder::new();
        enc.append_u8(PktS2Cid::HelloReply as u8);
        enc.append_sessionid(self.sid);
        enc.append_exhaustive_str(&self.username);
        return enc.consume();
    }
}
impl Encode for PktS2C_ReceiveMsg{
    fn encode(self) -> Vec<u8> {
        let mut enc = Encoder::new();
        enc.append_u8(PktS2Cid::ReceiveMsg as u8);
        enc.append_str(&self.msg);
        return enc.consume();
    }
}
impl Encode for PktS2C_SetNameReply{
    fn encode(self) -> Vec<u8> {
        let mut enc = Encoder::new();
        enc.append_u8(PktS2Cid::SetNameReply as u8);
        enc.append_str(&self.name);
        return enc.consume();
    }
}
impl Encode for PktS2C_LobbyInfo{
    fn encode(self) -> Vec<u8> {
        let mut enc = Encoder::new();
        enc.append_u8(PktS2Cid::LobbyInfo as u8);
        enc.append_uvarint(self.users.len() as u32);
        for u in self.users{
            enc.append_str(&u);
        }
        return enc.consume();
    }
}

// Universal decode function
pub fn decode(src: Vec<u8>) -> R<PktC2S>{
    let mut src = Decoder::new(src);
    let kind = src.get_u8()?;
    let Some(kind) = PktC2Sid::from_u8(kind) else { return Err(()) };
    use PktC2Sid::*;
    let result = match kind{
        Hello   => PktC2S_Hello::decode(&mut src)?.into(),
        SendMsg => PktC2S_SendMsg::decode(&mut src)?.into(),
        SetName => PktC2S_SetName::decode(&mut src)?.into(),
    };
    return Ok(result);
}
