use skyforest_rasterizer::api::create_router;

#[tokio::main]
async fn main() {
    let app = create_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind port");

    println!("🚀 TIFF Extractor API Server");
    println!("📡 Listening on http://0.0.0.0:3000");
    println!();
    println!("📍 Endpoints:");
    println!("  GET  /api/coordinate?latitude=<lat>&longitude=<lon>&tiff_path=<path>");
    println!("  POST /api/upload (multipart/form-data: csv file + tiff_path)");
    println!();

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
