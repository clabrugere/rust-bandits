mod actors;
mod api;
mod config;
mod policies;

use actix::prelude::*;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use actors::{cache::PolicyCache, supervisor::Supervisor};
use api::routes::{
    add_arm_bandit, bandit_stats, clear, create_bandit, delete_arm_bandit, delete_bandit,
    draw_bandit, list_bandits, reset_bandit, update_bandit, update_batch_bandit,
};
use config::AppConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = AppConfig::from_env().expect("Cannot read config");
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(config.server.log_level));

    let cache = PolicyCache::new(config.cache).start();
    let supervisor = Supervisor::new(config.supervisor, config.bandit, cache.clone()).start();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(supervisor.clone()))
            .wrap(Logger::default())
            .service(list_bandits)
            .service(clear)
            .service(create_bandit)
            .service(reset_bandit)
            .service(delete_bandit)
            .service(add_arm_bandit)
            .service(delete_arm_bandit)
            .service(draw_bandit)
            .service(update_bandit)
            .service(update_batch_bandit)
            .service(bandit_stats)
    })
    .bind((config.server.host, config.server.port))?
    .run()
    .await
}
