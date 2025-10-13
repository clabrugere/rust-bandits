mod actors;
mod api;
mod config;
mod errors;
mod policies;
mod repository;

use crate::api::routes::{disable_arm, enable_arm, ping_experiment, reset_arm};
use crate::repository::Repository;

use actix::prelude::*;
use actix_web::{
    middleware::from_fn,
    web::{scope, Data},
    App, HttpServer,
};
use actors::{accountant::Accountant, state_store::StateStore};
use api::responses::log_response;
use api::routes::{
    add_arm, clear, create, delete_arm, delete_experiment, draw, list, ping, reset, stats, update,
    update_batch,
};
use config::AppConfig;
use std::io::Error;
use tokio::sync::RwLock;
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    let config = AppConfig::from_env().expect("Failed to load configuration");

    // Initialize tracing subscriber with env filter
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.server.log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let accountant = Data::new(Accountant::new(config.accountant).start());
    let state_store = StateStore::new(config.state_store).start();
    let repository = Data::new(RwLock::new(Repository::new(
        config.experiment,
        state_store.clone(),
    )));

    // Reload all existing policies
    match repository.write().await.load_experiments().await {
        Ok(_) => (),
        Err(err) => {
            warn!(
                "Could not initialize experiment repository: {}",
                err.to_string()
            );
        }
    };

    HttpServer::new(move || {
        App::new()
            .app_data(accountant.clone())
            .app_data(repository.clone())
            .service(ping)
            .service(
                scope("/v1").service(
                    scope("/experiments")
                        .wrap(from_fn(log_response))
                        .service(list)
                        .service(clear)
                        .service(create)
                        .service(ping_experiment)
                        .service(reset)
                        .service(delete_experiment)
                        .service(add_arm)
                        .service(disable_arm)
                        .service(enable_arm)
                        .service(reset_arm)
                        .service(delete_arm)
                        .service(draw)
                        .service(update)
                        .service(update_batch)
                        .service(stats),
                ),
            )
    })
    .bind((config.server.host, config.server.port))?
    .run()
    .await
}
