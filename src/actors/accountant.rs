use crate::{api::responses::LoggedResponse, config::AccountantConfig};

use actix::{Actor, Context, Handler, Message};
use log::info;

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
        info!("Starting accountant");
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
        let serialized = serde_json::to_string(&msg.response).unwrap_or_default();
        info!("Logged response:\n{serialized}");
    }
}
