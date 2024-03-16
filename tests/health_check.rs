fn spawn_app() -> Result<(), std::io::Error> {
    let server = zero2prod::run()?;
    tokio::spawn(server);
    Ok(())
}

#[tokio::test]
async fn health_check_works() {
    spawn_app().expect("Failed to spawn the app.");

    let client = reqwest::Client::new();

    let respose = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute health check request");

    assert!(respose.status().is_success());
    assert_eq!(Some(0), respose.content_length());
}
