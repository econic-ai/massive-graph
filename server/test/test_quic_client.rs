//! Simple QUIC client to test the QUIC ingress service

use s2n_quic::client::Client;
use s2n_quic::provider::tls;
use s2n_quic::client::Connect;
use std::net::IpAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Starting QUIC client test...");
    
    // Create QUIC client
    let client = Client::builder()
        .with_tls(tls::default::Client::builder().build()?)?
        .with_io("0.0.0.0:0")?
        .start()?;
    
    // Connect to the server
    let connect = Connect::new((IpAddr::from([127, 0, 0, 1]), 4433))
        .with_server_name("localhost");
    println!("Connecting to QUIC server at 127.0.0.1:4433");
    
    let mut connection = client.connect(connect).await?;
    println!("Connected to QUIC server!");
    
    // Open a unidirectional send stream (like the server expects)
    let mut send_stream = connection.open_send_stream().await?;
    println!("Opened send stream");
    
    // Send a test message
    let test_message = b"Hello QUIC server! This is a test delta.";
    println!("Sending test message: {}", String::from_utf8_lossy(test_message));
    
    // Send the message
    send_stream.send(test_message.to_vec().into()).await?;
    println!("Message sent successfully");
    
    // Close the stream
    send_stream.close().await?;
    println!("Send stream closed");
    
    // Wait a bit for the server to process
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    println!("QUIC client test completed successfully!");
    Ok(())
}