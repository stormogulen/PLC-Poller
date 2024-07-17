use rand::Rng;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;

struct SharedState {
    current_id: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(SharedState { current_id: 0 }));

    // Spawn a task to update the ID every 10 seconds
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;
            let new_id = rand::thread_rng().gen_range(1..=200);
            {
                let mut state = state_clone.lock().await;
                state.current_id = new_id;
            }
            println!("New ID generated: {}", new_id);
        }
    });

    // Create and run the WebSocket server
    let listener = TcpListener::bind("127.0.0.1:5502").await?;
    println!("WebSocket server listening on 127.0.0.1:5502");

    while let Ok((stream, _)) = listener.accept().await {
        let state = state.clone();
        tokio::spawn(async move {
            let callback = |_req: &Request, mut response: Response| {
                response.headers_mut().insert("Connection", "upgrade".parse().unwrap());
                Ok(response)
            };

            match accept_hdr_async(stream, callback).await {
                Ok(ws_stream) => {
                    handle_connection(ws_stream, state).await;
                }
                Err(e) => {
                    eprintln!("Error during WebSocket handshake: {}", e);
                }
            }
        });
    }

    Ok(())
}

async fn handle_connection(
    stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    state: Arc<Mutex<SharedState>>,
) {
    let (mut write, mut read) = stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary( buf)) => {
                if buf.len() >= 12 {
                    let function_code = buf[7];
                    match function_code {
                        0x03 => {
                            let current_id = {
                                let state = state.lock().await;
                                state.current_id
                            };
                            let response = create_read_holding_registers_response(current_id);
                            if let Err(e) = write.send(Message::Binary(response)).await {
                                eprintln!("Failed to send response: {}", e);
                                break;
                            }
                        }
                        0x06 => {
                            let new_value = u16::from_be_bytes([buf[10], buf[11]]);
                            {
                                let mut state = state.lock().await;
                                state.current_id = new_value;
                            }
                            let response = create_write_single_register_response(&buf[0..6]);
                            if let Err(e) = write.send(Message::Binary(response)).await {
                                eprintln!("Failed to send response: {}", e);
                                break;
                            }
                        }
                        _ => {
                            eprintln!("Unsupported function code: {}", function_code);
                            break;
                        }
                    }
                } else {
                    eprintln!("Incomplete request");
                    break;
                }
            }
            Ok(Message::Frame(_)) => todo!(),
            Ok(Message::Text(_)) => eprintln!("Received unexpected text message"),
            Ok(Message::Ping(ping)) => {
                if let Err(e) = write.send(Message::Pong(ping)).await {
                    eprintln!("Failed to send Pong: {}", e);
                    break;
                }
            }
            Ok(Message::Pong(_)) => (),
            Ok(Message::Close(_)) => break,
            Err(e) => {
                eprintln!("Error processing message: {}", e);
                break;
            }
        }
    }
}

fn create_read_holding_registers_response(value: u16) -> Vec<u8> {
    let mut response = vec![
        0x00, 0x00, // Transaction ID (just echoing back 0)
        0x00, 0x00, // Protocol ID (0 for Modbus TCP)
        0x00, 0x05, // Length
        0x01,       // Unit ID (not used in Modbus TCP, set to 1)
        0x03,       // Function code (3 for Read Holding Registers)
        0x02,       // Byte count
    ];
    response.extend_from_slice(&value.to_be_bytes());
    response
}

fn create_write_single_register_response(request: &[u8]) -> Vec<u8> {
    let mut response = Vec::with_capacity(12);
    response.extend_from_slice(request); // Echo back the request header
    response.extend_from_slice(&[0x00, 0x06, 0x00, 0x00]); // Protocol ID and length
    response
}



