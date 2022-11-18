#![allow(dead_code)]
use std::io::{Error, Result};
use std::io::{Read, Write};
use std::net::*;
use std::sync::atomic::AtomicU32;

pub trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

pub enum VirtualMachineCommand {
    Version {
        description: String,
        major: i32,
        minor: i32,
        version: String,
        name: String,
    },
    Capabilities {
        watch_field_modification: bool,
        watch_field_access: bool,
        get_bytecodes: bool,
        get_synthetic_attribute: bool,
        get_owned_monitor_info: bool,
        get_current_contended_monitor: bool,
        get_monitor_info: bool,
    },
    CapabilitiesNew {
        watch_field_modification: bool,
        watch_field_access: bool,
        get_bytecodes: bool,
        get_synthetic_attribute: bool,
        get_owned_monitor_info: bool,
        get_current_contended_monitor: bool,
        get_monitor_info: bool,
        redefine_classes: bool,
        add_method: bool,
        unrestrictedly_redefine_classes: bool,
        pop_frames: bool,
        use_instance_filters: bool,
        get_source_debug_extension: bool,
        request_vm_death_event: bool,
        set_default_stratum: bool,
        get_instance_info: bool,
        request_monitor_events: bool,
        get_monitor_frame_info: bool,
        use_source_name_filters: bool,
        get_constant_pool: bool,
        force_early_return: bool,
        reserved22: bool,
        reserved23: bool,
        reserved24: bool,
        reserved25: bool,
        reserved26: bool,
        reserved27: bool,
        reserved28: bool,
        reserved29: bool,
        reserved30: bool,
        reserved31: bool,
        reserved32: bool,
    },
}

pub enum CommandSet {
    VirtualMachine(VirtualMachineCommand),
    ReferenceType,
    ClassType,
    ArrayType,
    InterfaceType,
    Method,
    Field,
    ObjectReference,
    StringReference,
    ThreadReference,
    ThreadGroupReference,
    ArrayReference,
    ClassLoaderReference,
    EventRequest,
    StackFrame,
    ClassObjectReference,
    Event,
}

pub struct Connection {
    pub stream: Box<dyn ReadWrite>,
    pub send_id: AtomicU32,    
}

impl Connection {
    pub fn new(stream: Box<dyn ReadWrite>) -> Connection {
        return Connection {
            stream: stream,
            send_id: AtomicU32::new(0),
        };
    }
    pub fn tcp<A: ToSocketAddrs>(addr: A) -> Result<Connection> {
        let stream = TcpStream::connect(addr)?;
        return Ok(Self::new(Box::new(stream)));
    }
}
