use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};

struct TestApp {
    address: String,
    db_pool: PgPool,
}

async fn spawn_app() -> Result<TestApp, std::io::Error> {
    // Port 0 is special-cased at the OS level: trying to bind port 0 will trigger an OS scan for
    // an available port which will then be bound to the application.
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("failed to read configuration");
    // randomise database name to make test cases independent
    configuration.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_db(&configuration.database).await;
    let server = zero2prod::startup::run(listener, connection_pool.clone())?;
    tokio::spawn(server);
    Ok(TestApp {
        address,
        db_pool: connection_pool,
    })
}

async fn configure_db(config: &DatabaseSettings) -> PgPool {
    // Create db
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("failed to connect to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("failed to create database");

    // Migrate db
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("failed to migrate the database");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    // Tokio runtime will kill the background task when the test finishes
    let app = spawn_app().await.expect("Failed to spawn the app.");
    let client = reqwest::Client::new();

    let url = format!("{}/health_check", &app.address);
    let respose = client
        .get(url)
        .send()
        .await
        .expect("Failed to execute health check request");

    assert!(respose.status().is_success());
    assert_eq!(Some(0), respose.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await.expect("Failed to spawn the app.");
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let url = format!("{}/subscriptions", &app.address);
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    let app = spawn_app().await.expect("Failed to spawn the app.");
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    let url = format!("{}/subscriptions", &app.address);
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "Expected 400 for payload of type: {}",
            error_message
        );
    }
}
