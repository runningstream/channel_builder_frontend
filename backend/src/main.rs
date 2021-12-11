#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate log;

pub mod schema;
pub mod db_models;
pub mod email;
pub mod helpers;
pub mod db;
pub mod api;
pub mod models;
pub mod api_handlers;
pub mod password_hash_version;

/// Returns the environment variable specified, or reads the value
/// out of the file specified in an environment variable, or returns
/// the default_val if that isn't None.
///
/// Panics if it cannot return a value.
fn get_env_param(param_name: &str, default_val: Option<&str>) -> String {
    let param_file_name = format!("{}_FILE", param_name.clone());

    match std::env::var(param_name) {
        Ok(val) => val,
        Err(_) => match std::env::var(param_file_name.clone()) {
            Ok(filename) => match std::fs::read_to_string(filename.clone()) {
                Ok(val) => val.trim().to_string(),
                Err(err) => panic!("Error reading {} file {}: {:?}", 
                    param_file_name, filename, err),
            }
            Err(_) => match default_val {
                Some(val) => val.to_string(),
                None => panic!("Value must be specified for env var {}", param_name),
            }
        }
    }
}

#[tokio::main]
async fn main() {

    // Get some parameters from the environment
    let db_password = get_env_param("POSTGRES_PASSWORD", None);
    let db_host = get_env_param("POSTGRES_HOST", Some("localhost:5432"));
    let server_address = get_env_param("CB_LISTEN", Some("127.0.0.1:3031"));
    let frontend_loc = get_env_param("FRONTEND_LOC",
        Some("http://localhost:8080"));
    let smtp_server = get_env_param("SMTP_SERVER", Some("localhost"));
    let smtp_port_str = get_env_param("SMTP_PORT", Some("25"));
    let smtp_username = get_env_param("SMTP_USERNAME", Some("webmaster"));
    let smtp_password = get_env_param("SMTP_PASSWORD", Some(""));
    let email_from = get_env_param("EMAIL_FROM_ADDR", Some("webmaster@localhost"));

    let smtp_port: u16 = match smtp_port_str.parse() {
        Ok(val) => val,
        Err(err) => panic!("Error parsing smtp_port: {}", err),
    };

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", format!("{}=info", api::LOG_KEY));
    }
    pretty_env_logger::init();

    // Setup DB with arc mutex
    let db_url = format!("postgres://{}:{}@{}/roku_channel_builder",
        "postgres", db_password, db_host);
    let db = db::Db::new(&db_url);

    // Setup email handler?
    let email = email::Email::new(smtp_server, smtp_port, smtp_username,
        smtp_password, email_from, frontend_loc.clone());

    info!("channel_builder version {}", helpers::VERSION);

    let api = api::build_filters(db, email, frontend_loc);
    let server_sockaddr: std::net::SocketAddr = server_address
        .parse()
        .expect("Unable to parse socket address");
    warp::serve(api).run(server_sockaddr).await;
}
