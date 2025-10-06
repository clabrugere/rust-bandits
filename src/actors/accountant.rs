#![allow(dead_code)]

use crate::{api::responses::LoggedResponse, config::AccountantConfig};

use actix::{Actor, Context, Handler, Message};
use tracing::{debug, info};

pub struct Accountant {
    config: AccountantConfig,
}

impl Accountant {
    pub fn new(config: AccountantConfig) -> Self {
        Self { config }
    }
}

impl Actor for Accountant {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Starting accountant actor");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LogResponse {
    pub response: LoggedResponse,
}

impl Handler<LogResponse> for Accountant {
    type Result = ();

    fn handle(&mut self, msg: LogResponse, _: &mut Self::Context) -> Self::Result {
        debug!(
            id = %msg.response.id,
            route = %msg.response.route,
            status = msg.response.status,
            timestamp = msg.response.timestamp,
            "Persisting log entry to storage"
        );

        //TODO: Implement database storage
        // Example with sqlx:
        // let pool = self.db_pool.clone();
        // let log = msg.response;
        // actix::spawn(async move {
        //     sqlx::query!(
        //         "INSERT INTO request_logs (id, timestamp, route, status) VALUES ($1, $2, $3, $4)",
        //         log.id, log.timestamp as i64, log.route, log.status as i16
        //     )
        //     .execute(&pool)
        //     .await
        // });
    }
}
