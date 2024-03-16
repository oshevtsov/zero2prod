fn spawn_app() -> Result<String, std::io::Error> {
    // Port 0 is special-cased at the OS level: trying to bind port 0 will trigger an OS scan for
    // an available port which will then be bound to the application.
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = zero2prod::run(listener)?;
    tokio::spawn(server);
    Ok(format!("http://127.0.0.1:{}", port))
}

#[tokio::test]
async fn health_check_works() {
    // Tokio runtime will kill the background task when the test finishes
    let address = spawn_app().expect("Failed to spawn the app.");

    let client = reqwest::Client::new();

    let url = format!("{}/health_check", address);
    let respose = client
        .get(url)
        .send()
        .await
        .expect("Failed to execute health check request");

    assert!(respose.status().is_success());
    assert_eq!(Some(0), respose.content_length());
}
