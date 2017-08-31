//! The main entrance for server functionality.

use AsCodec;
use chrono;
use futures::{Future, Stream};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;
use tokio_io::AsyncRead;

/// Run the server. The server will simply listen for new connections, receive
/// strings, and write them to STDOUT.
///
/// The function will block until the server is shutdown.
pub fn server() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let remote_addr = "127.0.0.1:14566".parse().unwrap();

    let listener = TcpListener::bind(&remote_addr, &handle).unwrap();

    // Accept all incoming sockets
    let server = listener.incoming().for_each(move |(socket, _)| {
        let transport = socket.framed(AsCodec::default());

        let process_connection = transport.for_each(|as_datum| {
            match as_datum.ts {
                Some(t) => {
                    let now = chrono::Utc::now().timestamp();
                    info!("latency: {:?}", now - t);
                }
                None => {}
            }
            Ok(())
        });

        // Spawn a new task dedicated to processing the connection
        handle.spawn(process_connection.map_err(|_| ()));
        Ok(())
    });

    // Open listener
    core.run(server).unwrap();
}