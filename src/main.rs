//! Notes API - Clean architecture example in Rust.
//!
//! This is the entry point for the application. It sets up the CLI, logging,
//! and starts the HTTP server.

mod adapter;
mod auth;
mod domain;
mod persistence;

use std::sync::Arc;

use adapter::web::NoteApi;
use clap::{Parser, Subcommand};
use persistence::memory::InMemoryNoteRepository;
use poem::{
    get, handler, http::StatusCode, listener::TcpListener, middleware::Cors, EndpointExt, Route,
    Server,
};
use poem_openapi::OpenApiService;
use tracing::instrument;

type NoteService = crate::domain::note::NoteService<InMemoryNoteRepository>;

#[derive(Debug, Parser)]
#[command(name = "notes-api")]
#[command(about = "A clean architecture note-taking API")]
struct AppArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Dump OpenAPI specification to stdout
    Spec,
    /// Start the server
    Server(ServerConfig),
}

#[derive(Debug, Parser)]
struct ServerConfig {
    #[clap(long, env = "HOST", default_value = "127.0.0.1")]
    host: String,

    #[clap(long, env = "PORT", default_value = "3000")]
    port: u16,

    #[clap(long, env = "JWT_SECRET", default_value = "dev-secret-change-me")]
    jwt_secret: String,

    /// Bypass JWT validation for development
    #[clap(long, env = "DEV_MODE", default_value = "false")]
    dev_mode: bool,

    #[clap(
        long,
        env = "CORS_ALLOWED_ORIGINS",
        default_value = "http://localhost:3000",
        value_delimiter = ','
    )]
    cors_allowed_origins: Vec<String>,
}

fn api_service() -> OpenApiService<NoteApi, ()> {
    OpenApiService::new(NoteApi, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .description("A clean architecture note-taking API")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("notes_api=debug".parse()?)
                .add_directive("poem=info".parse()?),
        )
        .init();

    let args = AppArgs::parse();

    match args.command {
        #[expect(clippy::print_stdout, reason = "CLI output for spec command")]
        Command::Spec => {
            println!("{}", api_service().spec());
        }
        Command::Server(config) => {
            server(config).await?;
        }
    }

    Ok(())
}

async fn server(config: ServerConfig) -> Result<(), std::io::Error> {
    tracing::info!(host = %config.host, port = %config.port, dev_mode = %config.dev_mode, "Starting server");

    let api_service = api_service().server(format!("http://{}:{}", config.host, config.port));

    let repository = InMemoryNoteRepository::new();
    let note_service = NoteService::new(repository);

    let auth_config = Arc::new(auth::AuthConfig {
        jwt_secret: config.jwt_secret.clone(),
        dev_mode: config.dev_mode,
    });

    let app = Route::new()
        .nest("/", api_service)
        .at("/_health/liveness", get(liveness))
        .at("/_health/readiness", get(readiness))
        .data(note_service)
        .data(auth_config)
        .with(cors(&config));

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!(address = %addr, "Server listening");

    Server::new(TcpListener::bind(&addr)).run(app).await
}

#[handler]
#[instrument]
fn liveness() -> StatusCode {
    StatusCode::OK
}

#[handler]
#[instrument]
fn readiness() -> StatusCode {
    StatusCode::OK
}

fn cors(config: &ServerConfig) -> Cors {
    Cors::new().allow_origins(config.cors_allowed_origins.iter().map(String::as_str))
}
