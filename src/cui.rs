use crate::{Result,Error};
use std::sync::mpsc::{self,Sender,Receiver,Iter};
use std::io::{BufReader,BufWriter,Read,Write};
use std::vec::Vec;
use log::*;
use std::collections::HashMap;
use std::default::Default;
use crate::jdwp;

#[derive(Default)]
struct State {
    id: u32,
    pub idsizes: jdwp::IDSizes,
    pub name: String,
    pub version: String,
    pub description: String,
    pub major: i32,
    pub minor: i32,
    pub capabilities: jdwp::Capabilities,
    pending: HashMap<u32, (u8, u8)>,
}

enum DeserializedPacket {
    Command(u32, jdwp::Command),
    Reply(u32, jdwp::Reply),
    Error(u32, jdwp::Error),
}

impl State {
    pub fn send_command<W: Write>(&mut self, cmd: &jdwp::Command, writer: &mut W) -> std::io::Result<()> {
        let (set, cmd, data) = cmd.serialize(self.idsizes);
        let id = self.id;
        let packet = jdwp::Packet::Command {
            id: id,
            set: set,
            cmd: cmd,
            data: data,
        };
        let res = packet.write(writer);
        if res.is_ok() {
            self.pending.insert(id, (set, cmd));
            self.id += 1;
        }
        return res;
    }
    pub fn send_reply<W: Write>(&mut self, id: u32, r: &jdwp::Result<jdwp::Reply>, writer: &mut W) -> std::io::Result<()> {
        return match r {
            Ok(reply) => {
                let data = reply.serialize(self.idsizes);
                let packet = jdwp::Packet::Reply {
                    id: id,
                    error: 0,
                    data: data,
                };
                packet.write(writer)
            },
            Err(e) => {
                let packet = jdwp::Packet::Reply {
                    id: id,
                    error: e.serialize(),
                    data: Vec::with_capacity(0),
                };
                packet.write(writer)
            }
        }
    }
    pub fn deserialize_packet(&mut self, packet: &jdwp::Packet) -> jdwp::Result<DeserializedPacket> {
        let deserialized = match packet {
            jdwp::Packet::Command { id, set, cmd, data } => {
               DeserializedPacket::Command(*id, jdwp::Command::deserialize(*set, *cmd, data.as_slice(), self.idsizes)?)
            },
            jdwp::Packet::Reply {id, error, data} => {
                if *error == 0u16 {
                    match self.pending.remove(id) {
                        Some((set, cmd)) => {
                            DeserializedPacket::Reply(
                                *id,
                                jdwp::Reply::deserialize(set, cmd, data.as_slice(), self.idsizes)?
                            )
                        },
                        None => {
                            error!("Got a reply packet with no corresponding command!");
                            return Err(jdwp::Error::IllegalArgument); 
                        },
                    }
                } else {
                    if self.pending.remove(id).is_none() {
                        warn!("Got an error packet with no corresponding command?");
                    }
                    match jdwp::Error::deserialize(*error) {
                        Some(e) => DeserializedPacket::Error(*id, e),
                        None => { return Err(jdwp::Error::Unimplemented); }
                    }
                }
            }
        };
        // Some information is cached so I don't have to go through
        // the pain of making another request for common inquiries.
        match &deserialized {
            DeserializedPacket::Reply(_, rply) => {
                match rply {
                    jdwp::Reply::Version { description, major, minor, version, name } => {
                        self.name = name.clone();
                        self.description = description.clone();
                        self.version = version.clone();
			self.major = *major;
                        self.minor = *minor;
                    },
                    jdwp::Reply::Capabilities(capabilities) => {
                        let mut capbits = self.capabilities.bits();
                        capbits = (capbits & !0x7f) | (capabilities.bits() & 0x7f);
                        self.capabilities = jdwp::Capabilities::from_bits(capbits).unwrap();
                    },
                    jdwp::Reply::CapabilitiesNew(capabilities) => {
                        self.capabilities = *capabilities;
                    },
                    jdwp::Reply::IDSizes { field, method, object, reference_type, frame } => {
                        self.idsizes = jdwp::IDSizes {
                            field: *field,
                            method: *method,
                            object: *object,
                            reference_type: *reference_type,
                            frame: *frame,
                        }
                    },
                    _ => {},
                }
            },
            _ => {}
        }
        return Ok(deserialized);
    }
    pub fn replies_left(&self) -> usize {
        return self.pending.len();
    }
    pub fn supports_version(&self, major: i32, minor: i32) -> bool {
        return self.major >= major && self.minor >= minor;
    }
}

fn event_thread<R: Read>(conn_data: R, vm_channel: Sender<jdwp::Packet>) -> Result<()> {
    let mut conn = conn_data;
    loop {
        let packet = jdwp::Packet::read(&mut conn);
        if packet.is_err() {
            error!("JDWP Read Error: {:?}", packet);
            break;
        }
        vm_channel.send(packet.unwrap());
    }
    Ok(())
}

fn recv_until_all_replied(vm_channel: &mut Receiver<jdwp::Packet>, state: &mut State) -> Vec<DeserializedPacket> {
    let mut packets: Vec<DeserializedPacket> = vec![];
    while state.replies_left() > 0 {
        let reply_packet = vm_channel.recv().expect("JDWP server closed connection while we were waiting for replies!");
        let deserialized = match state.deserialize_packet(&reply_packet) {
            Ok(p) => p,
            Err(e) => { error!("Failed to deserialize packet: {:?}", e); continue; },
        };
        packets.push(deserialized);
    }
    return packets;
}

fn prompt_thread<W: Write>(conn_const: W, vm_channel_const: Receiver<jdwp::Packet>) -> Result<()> {
    let mut conn = conn_const;
    let mut vm_channel = vm_channel_const;
    let mut state: State = Default::default();

    // We must know version, capabilities, and ID sizes before anything else
    state.send_command(&jdwp::Command::Version, &mut conn).unwrap();
    conn.flush();
    let _ = recv_until_all_replied(&mut vm_channel, &mut state);
    if state.supports_version(1, 4) {
        state.send_command(&jdwp::Command::CapabilitiesNew, &mut conn).unwrap();
    } else {
        state.send_command(&jdwp::Command::Capabilities, &mut conn).unwrap();
    }
    state.send_command(&jdwp::Command::IDSizes, &mut conn).unwrap();
    conn.flush();
    let _ = recv_until_all_replied(&mut vm_channel, &mut state);
    println!("VM Version: {}.{}", state.major, state.minor);
    println!("VM Name: {}", state.name);
    println!("VM Capabilities: {:?}", state.capabilities);
    println!("ID Sizes: {:?}", state.idsizes);
    
    loop {
        break;
    }
    Ok(())
}

pub fn main<R: Read + Send + 'static, W: Write + Send + 'static>(r: R, w: W) -> Result<()> {
    let mut bufread = BufReader::new(r);
    let mut bufwrite = BufWriter::new(w);
    let handshake_str = b"JDWP-Handshake";
    let mut handshake_buffer: [u8; 14] = [0u8; 14];
    let _ = &mut handshake_buffer.copy_from_slice(handshake_str);
    info!("Conducting handshake...");
    bufwrite.write_all(&handshake_buffer)?;
    bufwrite.flush()?;
    bufread.read_exact(&mut handshake_buffer)?;
    if &handshake_buffer != handshake_str {
        error!("JDWP Handshake failed! Exiting!");
        return Err(Error::HandshakeFailed(handshake_buffer.to_vec()));
    }
    info!("Handshake successful!: {:?}", std::str::from_utf8(&handshake_buffer));
    println!("Connected to JVM");
    let (event, prompt) = mpsc::channel();
    let evthread_handle = std::thread::spawn(move || {
        event_thread(bufread, event);
    });
    let _ = prompt_thread(bufwrite, prompt);
    Ok(())
}
