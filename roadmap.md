## Core features

- [x] Create/Delete Bandits: Support multiple bandit instances per client.
- [x] Add/Remove Arms: Dynamically adjust available choices.
- [x] Draw Arm (Decision Making): Low-latency selection of the optimal arm.
- [x] Update with Feedback: Reward updates with optional decay for non-stationary problems.
- [x] Batch Updates: Efficient ingestion of delayed feedback.
- [ ] Allow changing bandit parameters
- [ ] Serialize/deserialize bandit instances and load from existing state
    - the bandit actor only needs to implement Serialize + Deserialize (so each bandit policy as well). We implement a fn "save_state" that serialize the current state and push it to a storage
    - in the Actor's started fn, we register the "save_state" fn that checkpoint the state every x seconds:

```rust
impl Actor for BanditActor {
    fn started(&mut self, ctx: &mut Context<Self>) {
        // Periodically save state every 10 seconds
        ctx.run_interval(Duration::from_secs(10), |actor, _ctx| {
            actor.save_state();
        });
    }
}
```

    - in the Supervisor struct, implement a "restart_bandit" fn that check for an existing state and load it

```rust
impl Supervisor {
    fn restart_bandit(&mut self, bandit_id: Uuid) {
        if let Some(state) = self.storage.load_bandit(bandit_id) {
            if let Ok(policy) = serde_json::from_str::<BanditPolicy>(&state) {
                let actor = BanditActor::new(bandit_id, policy, Arc::clone(&self.storage)).start();
                self.bandits.insert(bandit_id, actor);
            }
        }
    }
}
```
    
    - in the bandit actor, send a message to the supervisor on stop (each bandit stores its ID and supervisor address):

```rust
impl actix::Actor for BanditActor {
    fn stopping(&mut self, _ctx: &mut Context<Self>) -> Running {
        self.supervisor.do_send(BanditCrashed { bandit_id: self.id });
        Running::Stop
    }
}
```

## Policies support

- [x] Epsilon Greedy
- [ ] UCB
- [ ] Thomson Sampling
- [ ] Optional reward decay for non-stationary environments

## Observability

- [x] Log every request and associated result with a unique id and timestamp
- [ ] Log every request and response to a database
- [x] Real time performance of bandits: pulls and rewards per arm
- [ ] Dashboard & metrics to visualize bandit performance: arm selection rates, conversion rates

## Scaling & performance

- [ ] Provide auth through API tokens
- [ ] Rate limiting on requests
- [ ] Checkpoint bandits state as recovery mechanism in case of crash