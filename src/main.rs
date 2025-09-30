mod actors;
mod api;
mod config;
mod errors;
mod policies;
mod repository;

use actix::prelude::*;
use actix_web::{
    middleware::{from_fn, Logger},
    web::{scope, Data},
    App, HttpServer,
};
use actors::{accountant::Accountant, experiment_cache::ExperimentCache};
use api::responses::log_response;
use api::routes::{
    add_arm, clear, create, delete_arm, delete_experiment, draw, list, ping, reset, stats, update,
    update_batch,
};
use config::AppConfig;
use log::warn;
use std::io::Error;
use std::sync::RwLock;

use crate::{
    api::routes::{ping_experiment, reset_arm},
    repository::Repository,
};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    let config = AppConfig::from_env().expect("Failed to load configuration");
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(config.server.log_level));

    let accountant = Data::new(Accountant::new(config.accountant).start());
    let policy_cache = ExperimentCache::new(config.experiment_cache).start();
    let repository = Data::new(RwLock::new(Repository::new(
        config.experiment,
        policy_cache.clone(),
    )));

    match repository
        .write()
        .map_err(|err| Error::new(std::io::ErrorKind::Other, err.to_string()))?
        .load_experiments()
        .await
    {
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
            .wrap(Logger::default())
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
                        .service(reset_arm)
                        .service(delete_experiment)
                        .service(add_arm)
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
