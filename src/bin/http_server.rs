//! Simple HTTP server runner for testing
//! Starts the AeroDB HTTP server on port 54321

use aerodb::http_server::HttpServer;

#[tokio::main]
async fn main() {
    println!("Starting AeroDB HTTP Server for testing...");
    
    let server = HttpServer::new();
    
    if let Err(e) = server.start().await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
