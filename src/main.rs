pub mod jdwp;
pub mod cui;
use std::net::*;
use std::io::{Write,Read};

#[derive(Debug)]
pub enum Error {
    HandshakeFailed(Vec<u8>),
    Io(std::io::Error),
    Jdwp(jdwp::Error),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        return Error::Io(e);
    }
}

impl From<jdwp::Error> for Error {
    fn from(e: jdwp::Error) -> Error {
        return Error::Jdwp(e);
    }
}

type Result<T> = std::result::Result<T, Error>;

pub fn tcp<A: ToSocketAddrs>(addr: A) -> Result<(TcpStream, TcpStream)> {
    let recv_end = TcpStream::connect(addr)?;
    let send_end = recv_end.try_clone()?;
    return Ok((recv_end,send_end));
}

fn main() -> Result<()> {
    env_logger::init();
    println!("Opening connection to localhost:4444!");
    let (r,w) = tcp("127.0.0.1:4444")?;
    cui::main(r,w)?; 
    Ok(())
}
