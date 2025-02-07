mod actors;
mod api;
mod config;
mod policies;

use actix::prelude::*;
use actix_web::{
    middleware::Logger,
    web::{scope, Data},
    App, HttpServer,
};
use actors::{policy_cache::PolicyCache, supervisor::Supervisor};
use api::routes::{
    add_arm, clear, create, delete, delete_arm, draw, list, reset, stats, update, update_batch,
};
use config::AppConfig;
use std::io::Error;

#[actix_web::main]
async fn main() -> Result<(), Error> {
    let config = AppConfig::from_env().expect("Failed to load configuration");
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(config.server.log_level));

    let cache = PolicyCache::new(config.cache).start();
    let supervisor = Supervisor::new(config.supervisor, config.bandit, cache.clone()).start();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(supervisor.clone()))
            .wrap(Logger::default())
            .service(
                scope("/bandits")
                    .service(list)
                    .service(clear)
                    .service(create)
                    .service(reset)
                    .service(delete)
                    .service(add_arm)
                    .service(delete_arm)
                    .service(draw)
                    .service(update)
                    .service(update_batch)
                    .service(stats),
            )
    })
    .bind((config.server.host, config.server.port))?
    .run()
    .await
}
