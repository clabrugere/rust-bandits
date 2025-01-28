mod actor;
mod api;
mod policies;
mod supervisor;

use actix::prelude::*;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use supervisor::Supervisor;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let supervisor = Supervisor::new().start();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(supervisor.clone()))
            .wrap(Logger::default())
            .service(api::list_bandits)
            .service(api::create_bandit)
            .service(api::reset_bandit)
            .service(api::delete_bandit)
            .service(api::add_arm_bandit)
            .service(api::delete_arm_bandit)
            .service(api::draw_bandit)
            .service(api::update_bandit)
            .service(api::bandit_stats)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
