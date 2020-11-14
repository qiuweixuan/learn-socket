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

#![warn(rust_2018_idioms)]


use tokio::io;
use tokio_util::codec::{ FramedRead, FramedWrite,LinesCodec};

use std::env;
use std::error::Error;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Determine if we're going to run in TCP or UDP mode
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let tcp = match args.iter().position(|a| a == "--udp") {
        Some(i) => {
            args.remove(i);
            false
        }
        None => true,
    };

    // Parse what address we're going to connect to
    let addr = args
        .first()
        .ok_or("this program requires at least one argument")?;
    let addr = addr.parse::<SocketAddr>()?;

    let stdin = FramedRead::new(io::stdin(), LinesCodec::new());
    // let stdin = stdin.map(|i| i.map(|bytes| bytes.freeze()));
    let stdout = FramedWrite::new(io::stdout(), LinesCodec::new());


    if tcp {
        tcp::connect(&addr, stdin, stdout).await?;
    }

    Ok(())
}

mod tcp {
    use futures::{future, Sink, SinkExt, Stream, StreamExt};
    use std::{error::Error, net::SocketAddr};
    use tokio::net::TcpStream;
    use tokio_util::codec::{ FramedRead, FramedWrite,LinesCodec,LinesCodecError};

    pub async fn connect(
        addr: &SocketAddr,
        mut stdin: impl Stream<Item = Result<String,LinesCodecError>> + Unpin,
        mut stdout: impl Sink< String, Error = LinesCodecError> + Unpin,
    ) -> Result<(), Box<dyn Error>> {
        let mut stream = TcpStream::connect(addr).await?;
        let (r, w) = stream.split();
        let mut sink = FramedWrite::new(w, LinesCodec::new());
        // filter map Result<BytesMut, Error> stream into just a Bytes stream to match stdout Sink
        // on the event of an Error, log the error and end the stream
        let mut stream = FramedRead::new(r, LinesCodec::new())
            .filter_map(|i| match i {
                //BytesMut into Bytes
                Ok(i) => future::ready(Some(i)),
                Err(e) => {
                    println!("failed to read from socket; error={}", e);
                    future::ready(None)
                }
            })
            .map(Ok);

        match future::join(sink.send_all(&mut stdin), stdout.send_all(&mut stream)).await {
            (Err(e), _) | (_, Err(e)) => Err(e.into()),
            _ => Ok(()),
        }
    }
}

