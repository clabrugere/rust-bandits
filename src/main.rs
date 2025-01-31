mod api;
mod bandit;
mod policy;
mod supervisor;

use actix::prelude::*;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use api::routes::{
    add_arm_bandit, bandit_stats, create_bandit, delete_arm_bandit, delete_bandit, draw_bandit,
    list_bandits, reset_bandit, update_bandit,
};
use supervisor::Supervisor;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let supervisor = Supervisor::new().start();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(supervisor.clone()))
            .wrap(Logger::default())
            .service(list_bandits)
            .service(create_bandit)
            .service(reset_bandit)
            .service(delete_bandit)
            .service(add_arm_bandit)
            .service(delete_arm_bandit)
            .service(draw_bandit)
            .service(update_bandit)
            .service(bandit_stats)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
