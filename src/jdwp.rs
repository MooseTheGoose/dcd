#![allow(dead_code)]
use bitflags::bitflags;
use std::vec::Vec;
use std::io::{Read,Write};
use log::*;

#[derive(Debug)]
pub enum Error {
    IllegalArgument,
    AbsentInformation,
    InvalidLength,
    InvalidString,
    Unimplemented,
}

impl Error {
    pub fn serialize(&self) -> u16 {
        return match self {
            Error::IllegalArgument => 103,
            Error::AbsentInformation => 101,
            Error::InvalidLength => 504,
            Error::InvalidString => 506,
            Error::Unimplemented => 99,
        };
    }
    pub fn deserialize(data: u16) -> Option<Error> {
        return Some(match data {
            103 => Error::IllegalArgument,
            101 => Error::AbsentInformation,
            504 => Error::InvalidLength,
            506 => Error::InvalidString,
            99 => Error::Unimplemented,
            _ => { return None; }
        });
    }
}

#[derive(Debug)]
pub enum Tag {
    Array(u64),
    Byte(u8),
    Char(u16),
    Object(u64),
    Float(f32),
    Double(f64),
    Int(i32),
    Long(i64),
    Short(i16),
    Void,
    Boolean(bool),
    String(u64),
    Thread(u64),
    ThreadGroup(u64),
    ClassLoader(u64),
    ClassObject(u64),
}
/*
pub enum ArrayRegion {
    Array(Vec<u64>),
    Byte(Vec<u8>),
    Char(Vec<u16>),
    Object(Vec<u64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    Int(Vec<i32>),
    Long(Vec<i64>),
    Short(Vec<i16>),
    Void,
    Boolean(Vec<bool>),
    String(Vec<u64>),
    Thread(Vec<u64>),
    ThreadGroup(Vec<u64>),
    ClassLoader(Vec<u64>),
    ClassObject(Vec<u64>),
}
*/

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone,Copy,Default,Debug)]
pub struct IDSizes {
    pub field: i32,
    pub method: i32,
    pub object: i32,
    pub reference_type: i32,
    pub frame: i32,
}

pub struct Serializer(pub Vec<u8>, pub IDSizes);

impl Serializer {
    pub fn write_untagged(&mut self, data_orig: u64, size: i32) {
        let data = data_orig;
        for i in 0..size {
            let b = (data >> (8 * (size - 1 - i)) & 0xff) as u8;
            self.0.push(b);
        }
    }
    pub fn write_array<T>(&mut self, data: &[T]) {
        let (_, data_u8, _) = unsafe { data.align_to() };
        self.0.extend_from_slice(data_u8);
    }
    pub fn write_ids(&mut self, data: &[u64], size: i32) {
        for d in data {
            self.write_untagged(*d, size);
        }
    }
    pub fn serialize_byte(&mut self, data: u8) {
        self.write_untagged(data as u64, 1);
    }
    pub fn serialize_bool(&mut self, data: bool) {
        self.serialize_byte(if data { 1u8 } else { 0u8 });
    }
    pub fn serialize_char(&mut self, data: u16) {
        self.write_untagged(data as u64, 2);
    }
    pub fn serialize_int(&mut self, data: i32) {
        self.write_untagged(data as u64, 4);
    }
    pub fn serialize_long(&mut self, data: i64) {
        self.write_untagged(data as u64, 8);
    }
    pub fn serialize_float(&mut self, data: f32) {
        self.write_untagged(data.to_bits() as u64, 4);
    }
    pub fn serialize_double(&mut self, data: f64) {
        self.write_untagged(data.to_bits() as u64, 8);
    }
    pub fn serialize_object(&mut self, id: u64) {
        self.write_untagged(id, self.1.object);
    }
    pub fn serialize_string(&mut self, s: &String) {
        let sbytes = s.as_bytes();
        let slen = sbytes.len();
        self.write_untagged(slen as u64, 4);
        self.write_array(sbytes);
    }
}

pub struct Deserializer<R: Read>(pub R, pub IDSizes);

impl<R: Read> Deserializer<R> {
    pub fn read_untagged(&mut self, size: i32) -> Result<u64> {
        let mut arr = [0u8; 8];
        if size <= 0 || size > 8 {
            return Err(Error::IllegalArgument);
        }
        return match self.0.read_exact(&mut arr[(8-size) as usize..]) {
            Ok(_) => Ok(u64::from_be_bytes(arr)),
            Err(_) => Err(Error::AbsentInformation),
        };
    }
    /*
    pub fn read_tagged(&mut self) -> Result<Tag> {
        let tag = self.deserialize_byte()?;
        return Ok(match tag {
            91 => Tag::Array(self.deserialize_object()?),
            66 => Tag::Byte(self.deserialize_byte()?),
            67 => Tag::Char(self.deserialize_char()?),
            76 => Tag::Object(self.deserialize_object()?),
            _ => { return Err(Error::IllegalArgument); } 
        });
    }
    */
    /*
    pub fn read_tagged_array(&mut self) -> Result<Vec<Tag>> {
        if size < 0 {
            return Err(Error::IllegalArgument);
        }
        let data: Vec<Tag> = Vec::with_capacity(size as usize);
        for i in 0..size {
        }
    }
    */
    pub fn read_array<T>(&mut self, size: usize) -> Result<Vec<T>> {
        let mut data: Vec<T> = Vec::with_capacity(size);
        unsafe { data.set_len(size); }
        let (_, data_u8, _) = unsafe { data.align_to_mut() };
        if self.0.read_exact(data_u8).is_err() {
            return Err(Error::AbsentInformation);
        }
        return Ok(data);
    }
    pub fn deserialize_byte(&mut self) -> Result<u8> {
        return Ok(self.read_untagged(1)? as u8);
    }
    pub fn deserialize_boolean(&mut self) -> Result<bool> {
        return Ok(self.deserialize_byte()? != 0);
    }
    pub fn deserialize_char(&mut self) -> Result<u16> {
        return Ok(self.read_untagged(2)? as u16);
    }
    pub fn deserialize_int(&mut self) -> Result<i32> {
        return Ok(self.read_untagged(4)? as i32);
    }
    pub fn deserialize_long(&mut self) -> Result<i64> {
        return Ok(self.read_untagged(8)? as i64);
    }
    pub fn deserialize_float(&mut self) -> Result<f32> {
        return Ok(f32::from_bits(self.read_untagged(4)? as u32));
    }
    pub fn deserialize_double(&mut self) -> Result<f64> {
        return Ok(f64::from_bits(self.read_untagged(8)? as u64));
    }
    pub fn deserialize_object(&mut self) -> Result<u64> {
        return Ok(self.read_untagged(self.1.object)?);
    }
    pub fn deserialize_string(&mut self) -> Result<String> {
        let length = self.read_untagged(4)? as i32;
        if length < 0 {
            return Err(Error::InvalidLength);
        }
        let data = self.read_array(length as usize)?;
        return match String::from_utf8(data) {
            Ok(s) => Ok(s),
            Err(_) => Err(Error::InvalidString),
        };
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Capabilities: u32 {
        const WATCH_FIELD_MODIFICATION = 0x1;
        const WATCH_FIELD_ACCESS = 0x2;
        const GET_BYTECODES = 0x4;
        const GET_SYNTHETIC_ATTRIBUTE = 0x8;
        const GET_OWNED_MONITOR_INFO = 0x10;
        const GET_CURRENT_CONTENDED_MONITOR = 0x20;
        const GET_MONITOR_INFO = 0x40;
        const REDEFINE_CLASSES = 0x80;
        const ADD_METHOD = 0x100;
        const UNRESTRICTEDLY_REDEFINE_CLASSES = 0x200;
        const POP_FRAMES = 0x400;
        const USE_INSTANCE_FILTERS = 0x800;
        const GET_SOURCE_DEBUG_EXTENSION = 0x1000;
        const REQUEST_VM_DEATH_EVENT = 0x2000;
        const SET_DEFAULT_STRATUM = 0x4000;
        const GET_INSTANCE_INFO = 0x8000;
        const REQUEST_MONITOR_EVENTS = 0x10000;
        const GET_MONITOR_FRAME_INFO = 0x20000;
        const USE_SOURCE_NAME_FILTERS = 0x40000;
        const GET_CONSTANT_POOL = 0x80000;
        const FORCE_EARLY_RETURN = 0x100000;
        const RESERVED = 0xffe00000;
    }
}

#[derive(Debug)]
pub enum Reply {
    Version {
        description: String,
        major: i32,
        minor: i32,
        version: String,
        name: String,
    },
    Capabilities(Capabilities),
    CapabilitiesNew(Capabilities),
    IDSizes {
        field: i32,
        method: i32,
        object: i32,
        reference_type: i32,
        frame: i32,
    },
}

impl Reply {
    pub fn serialize(&self, sizes: IDSizes) -> Vec<u8> {
        let mut serializer = Serializer(Vec::new(), sizes);
        match self {
            Reply::Version { description, major, minor, version, name } => {
                serializer.serialize_string(description);
                serializer.serialize_int(*major);
                serializer.serialize_int(*minor);
                serializer.serialize_string(version);
                serializer.serialize_string(name);
            },
            Reply::Capabilities(capabilities) => {
                let bits = capabilities.bits();
                for i in 0..7 { serializer.serialize_byte((bits >> i & 1) as u8); }
            }
            Reply::CapabilitiesNew(capabilities) => {
                 let bits = capabilities.bits();
                 for i in 0..21 { serializer.serialize_byte((bits >> i & 1) as u8); }
            }
            Reply::IDSizes {field, method, object, reference_type, frame } => {
                serializer.serialize_int(*field);
                serializer.serialize_int(*method);
                serializer.serialize_int(*object);
                serializer.serialize_int(*reference_type);
                serializer.serialize_int(*frame);
            }
        }
        return serializer.0; 
    }
    pub fn deserialize(set: u8, cmd: u8, data: &[u8], sizes: IDSizes) -> Result<Reply> {
        let mut deserializer = Deserializer(data, sizes);
        return Ok(match set {
            1 => match cmd {
                1 => Reply::Version {
                    description: deserializer.deserialize_string()?,
                    major: deserializer.deserialize_int()?,
                    minor: deserializer.deserialize_int()?,
                    version: deserializer.deserialize_string()?,
                    name: deserializer.deserialize_string()?,
                },
                12 => { 
                    let mut capabilities = 0u32;
                    for i in 0..7 {
                        if deserializer.deserialize_boolean()? {
                            capabilities |= 1 << i;
                        }
                    }
                    Reply::Capabilities(Capabilities::from_bits(capabilities).unwrap())
                },
                17 => {
                    let mut capabilities = 0u32;
                    for i in 0..21 {
                        if deserializer.deserialize_boolean()? {
                            capabilities |= 1 << i;
                        }
                    }
                    Reply::CapabilitiesNew(Capabilities::from_bits(capabilities).unwrap())
                },
                7 => Reply::IDSizes {
                    field: deserializer.deserialize_int()?,
                    method: deserializer.deserialize_int()?,
                    object: deserializer.deserialize_int()?,
                    reference_type: deserializer.deserialize_int()?,
                    frame: deserializer.deserialize_int()?,
                },
                _ => { return Err(Error::Unimplemented); },
            },
            _ => { return Err(Error::Unimplemented); },
        });
    }
}

#[derive(Debug)]
pub enum Command {
    Version,
    Capabilities,
    CapabilitiesNew,
    IDSizes,
}

impl Command {
    pub fn deserialize(set: u8, cmd: u8, data: &[u8], sizes: IDSizes) -> Result<Command> { 
        let _deserializer = Deserializer(data, sizes);
        return Ok(match set {
            1 => match cmd {
                1 => Command::Version,
                12 => Command::Capabilities,
                17 => Command::CapabilitiesNew,
                7 => Command::IDSizes,
                _ => { return Err(Error::Unimplemented) },
            },
            _ => { return Err(Error::Unimplemented); },
        });
    }
    pub fn serialize(&self, sizes: IDSizes) -> (u8, u8, Vec<u8>) {
        let serializer = Serializer(Vec::new(), sizes);
        let (set, cmd) = match self {
            Command::Version => (1, 1),
            Command::Capabilities => (1, 12),
            Command::CapabilitiesNew => (1, 17),
            Command::IDSizes => (1, 7),
        };
        return (set, cmd, serializer.0);
    }
}

pub struct Header {
    len: u32,
    id: u32,
    flags: u8,
}

#[derive(Debug)]
pub enum Packet {
    Command {
        id: u32,
        set: u8,
        cmd: u8,
        data: Vec<u8>, 
    },
    Reply {
        id: u32,
        error: u16,
        data: Vec<u8>,
    },
}

impl Packet {
    pub fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            Packet::Command { id, set, cmd, data } => {
                writer.write_all(&((data.len() + 11) as u32).to_be_bytes())?;
                writer.write_all(&id.to_be_bytes())?;
                writer.write_all(&[0u8, *set, *cmd])?;
                writer.write_all(data.as_slice())?;
            },
            Packet::Reply { id, error, data } => {
                writer.write_all(&((data.len() + 11) as u32).to_be_bytes())?;
                writer.write_all(&id.to_be_bytes())?;
                let errbytes = [(*error >> 8) as u8, (*error & 0xff) as u8];
                writer.write_all(&[0x80u8, errbytes[0], errbytes[1]])?;
                writer.write_all(data.as_slice())?;
            }
        }
        return Ok(());
    }
    pub fn read<R: Read>(reader: &mut R) -> std::io::Result<Packet> {
        let mut header = [0u8; 11];
        reader.read_exact(&mut header)?;
        let len = u32::from_be_bytes(header[0..4].try_into().unwrap());
        let id = u32::from_be_bytes(header[4..8].try_into().unwrap());
        let flags = header[8];
        if len < 11 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "JDWP packet length < 4"));
        }
        let datalen = (len - 11) as usize;
        let mut data = Vec::with_capacity(datalen);
        unsafe { data.set_len(datalen); }
        reader.read_exact(&mut data.as_mut_slice())?;
        return Ok(if (flags & 0x80u8) != 0u8 {
            Packet::Reply {
                id: id,
                error: ((header[9] as u16) << 8 | header[10] as u16),
                data: data,
            }
        } else {
            Packet::Command {
                id: id,
                set: header[9],
                cmd: header[10],
                data: data,
            }
        });
    }
}
