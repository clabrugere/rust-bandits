## TODO

- [x] BanditStats: return the mean reward instead of the cumulative reward
- [x] log requests with uuid, timestamp, endpoint requested and hash of the payload.
- [ ] for end-users, use "experiment" naming in place of "bandit" (for example in routes and structured responses)
- [ ] implement DB actor to log requests and responses
- [ ] cache persist: investigate append-only to keep some history

## Core features

- [x] Create/Delete Bandits: Support multiple bandit instances per client.
- [x] Add/Remove Arms: Dynamically adjust available choices.
- [x] Draw Arm (Decision Making): Low-latency selection of the optimal arm.
- [x] Update with Feedback: Reward updates with optional decay for non-stationary problems.
- [x] Batch Updates: Efficient ingestion of delayed feedback.
- [x] Serialize/deserialize bandit instances and load from existing state
- [ ] Allow changing bandit parameters

## Policies

- [x] Epsilon Greedy
- [ ] UCB
- [ ] Thomson Sampling
- [ ] Optional reward decay for non-stationary environments

## Observability

- [x] Log every request and associated result with a unique id and timestamp
- [x] Real time performance of bandits: pulls and average rewards per arm
- [ ] Log every request and response to a database
- [ ] Build a metric collection system (Prometheus + Grafana) to log count of requests, latency, active bandits, etc
- [ ] Dashboard & metrics to visualize bandit performance: arm selection rates, conversion rates

## Scaling & performance

- [x] Checkpoint bandits state as recovery mechanism in case of crash
- [ ] Provide auth through API tokens
- [ ] Rate limiting on requests


### old
```
pub async fn log_response<T: Debug + Clone + Send + Serialize + 'static>(
    accountant: Data<Addr<Accountant>>,
    route: &str,
    response: impl Future<Output = Result<Option<T>, ApiResponseError>>,
) -> Result<impl Responder> {
    match response.await {
        Ok(response) => {
            accountant.do_send(LogResponse {
                response: LoggedResponse::with_data(
                    route,
                    StatusCode::OK.as_u16(),
                    response.clone(),
                ),
            });

            match response {
                Some(data) => Ok(HttpResponse::Ok().json(data)),
                None => Ok(HttpResponse::Ok().finish()),
            }
        }
        Err(err) => {
            accountant.do_send(LogResponse {
                response: LoggedResponse::<()>::empty(route, err.status_code().as_u16()),
            });
            Err(err.into())
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct LoggedResponse<T: Clone> {
    route: String,
    request_id: Uuid,
    ts: u128,
    status_code: u16,
    data: Option<T>,
}

impl<T: Clone + Serialize> LoggedResponse<T> {
    fn new(route: &str, status_code: u16, data: Option<T>) -> Self {
        Self {
            route: route.to_string(),
            request_id: Uuid::new_v4(),
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            status_code,
            data,
        }
    }

    pub fn empty(route: &str, status_code: u16) -> Self {
        Self::new(route, status_code, None)
    }

    pub fn with_data(route: &str, status_code: u16, data: T) -> Self {
        Self::new(route, status_code, Some(data))
    }
}
```