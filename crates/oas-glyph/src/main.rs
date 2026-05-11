use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3627); // "OAS" on phone keypad ≈ 627, prefixed with 3

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    oas_glyph::api::init_start_time();
    let app = oas_glyph::api::router();

    println!("╔══════════════════════════════════════════╗");
    println!("║      OpenAgent Glyph Service v0.1.0      ║");
    println!("║   Visual Identity for the Open Agent Std  ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Listening on http://{}         ║", addr);
    println!("╚══════════════════════════════════════════╝");

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
