//! An example of hooking up stdin/stdout to either a TCP or UDP stream.
//!
//! This example will connect to a socket address specified in the argument list
//! and then forward all data read on stdin to the server, printing out all data
//! received on stdout. An optional `--udp` argument can be passed to specify
//! that the connection should be made over UDP instead of TCP, translating each
//! line entered on stdin to a UDP packet to be sent to the remote address.
//!
//! Note that this is not currently optimized for performance, especially
//! around buffer management. Rather it's intended to show an example of
//! working with a client.
//!
//! This example can be quite useful when interacting with the other examples in
//! this repository! Many of them recommend running this as a simple "hook up
//! stdin/stdout to a server" to get up and running.

//!     cargo run --example print_each_packet
//!     cargo run --example connect 127.0.0.1:8080

#![warn(rust_2018_idioms)]

use futures::StreamExt;
use tokio::io;
use tokio::prelude::*;
use tokio_util::codec::{FramedRead, LinesCodec};

use std::env;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::try_join;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Determine if we're going to run in TCP or UDP mode
    let args = env::args().skip(1).collect::<Vec<_>>();

    // Parse what address we're going to connect to
    let addr = args
        .first()
        .ok_or("this program requires at least one argument")?;
    let addr = addr.parse::<SocketAddr>()?;

    let server_sock = TcpStream::connect(addr).await?;
    let (read_half, mut write_half) = server_sock.into_split();

    let read_join = tokio::spawn(async move {
        let mut stdin = FramedRead::new(io::stdin(), LinesCodec::new());
        while let Some(message) = stdin.next().await {
            match message {
                // Ok(bytes) => { println!("bytes: {:x?}  ", bytes.as_bytes()) },
                Ok(mut bytes) => {
                    // println!("bytes: {:?}  ", bytes);
                    bytes.push('\n');
                    write_half.write_all(bytes.as_bytes()).await.unwrap();
                }
                Err(err) => println!("closed with error: {:?}", err),
            }
        }
    });
    let write_join = tokio::spawn(async move {
        let mut read_server = FramedRead::new(read_half, LinesCodec::new());
        while let Some(message) = read_server.next().await {
            match message {
                // Ok(bytes) => { println!("bytes: {:x?}  ", bytes.as_bytes()) },
                Ok(bytes) => {
                    println!("echo: {:?}  ", bytes);
                }
                Err(err) => println!("closed with error: {:?}", err),
            }
        }
    });
    /* match try_join!(read_join,write_join){
        Err(e) => {
            println!("read_join or write_join failed, error={}", e);
        },
        Ok(_) => {} ,
    }; */

    if let Err(e) = try_join!(read_join, write_join) {
        println!("read_join or write_join failed, error={}", e);
    };

    Ok(())
}
