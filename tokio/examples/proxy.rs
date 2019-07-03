//! A proxy that forwards data to another server and forwards that server's
//! responses back to clients.
//!
//! Because the Tokio runtime uses a thread pool, each TCP connection is
//! processed concurrently with all other TCP connections across multiple
//! threads.
//!
//! You can showcase this by running this in one terminal:
//!
//!     cargo run --example proxy
//!
//! This in another terminal
//!
//!     cargo run --example echo
//!
//! And finally this in another terminal
//!
//!     cargo run --example connect 127.0.0.1:8081
//!
//! This final terminal will connect to our proxy, which will in turn connect to
//! the echo server, and you'll be able to see data flowing between them.

#![feature(async_await)]

use futures::future::try_join;
use futures::prelude::StreamExt;
use std::env;
use std::net::SocketAddr;
use tokio;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::split::{TcpStreamReadHalf, TcpStreamWriteHalf};
use tokio::net::tcp::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listen_addr = env::args().nth(1).unwrap_or("127.0.0.1:8081".to_string());
    let listen_addr = listen_addr.parse::<SocketAddr>()?;

    let server_addr = env::args().nth(2).unwrap_or("127.0.0.1:8080".to_string());
    let server_addr = server_addr.parse::<SocketAddr>()?;

    // Create a TCP listener which will listen for incoming connections.
    let socket = TcpListener::bind(&listen_addr)?;
    println!("Listening on: {}", listen_addr);
    println!("Proxying to: {}", server_addr);

    let mut incoming = socket.incoming();
    loop {
        let stream = incoming.next().await.unwrap()?;
        tokio::spawn(async move {
            match proxy_client(stream, server_addr).await {
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
                _ => (),
            }
        });
    }
}

async fn proxy_client(
    client_stream: TcpStream,
    server_addr: SocketAddr,
) -> Result<(), std::io::Error> {
    let server_stream = TcpStream::connect(&server_addr).await?;

    // Create separate read/write handles for the TCP clients that we're
    // proxying data between.
    //
    // Note that while you can use `AsyncRead::split` for this operation,
    // `TcpStream::split` gives you handles that are faster, smaller and allow
    // proper shutdown operations.
    let (client_r, client_w) = client_stream.split();
    let (server_r, server_w) = server_stream.split();

    let client_to_server = copy_shutdown(client_r, server_w);
    let server_to_client = copy_shutdown(server_r, client_w);

    // Run the two futures in parallel.
    let (l1, l2) = try_join(client_to_server, server_to_client).await?;
    println!("client wrote {} bytes and received {} bytes", l1, l2);
    Ok(())
}

// Copy data from a read half to a write half. After the copy is done we
// indicate to the remote side that we've finished by shutting down the
// connection.
async fn copy_shutdown(
    mut r: TcpStreamReadHalf,
    mut w: TcpStreamWriteHalf,
) -> Result<u64, std::io::Error> {
    let l = r.copy(&mut w).await?;

    // Use this instead after `shutdown` is implemented in `AsyncWriteExt`:
    // w.shutdown().await?;
    w.as_ref().shutdown(std::net::Shutdown::Write)?;

    Ok(l)
}
