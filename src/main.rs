pub mod jdwp;
use std::process::ExitCode;

fn main() -> ExitCode {
    /*
    println!("Opening connection to localhost:4444!");
    let jdwp = jdwp::Connection::tcp("127.0.0.1:4444").expect("Failed to connect");
    */
    return ExitCode::from(0);
}
