# rust-bandits (WIP)

## About

This is a prototype of a backend application for real-time online experimentation based on stateful multi-armed bandits policies, written in rust. It's like AB testing, but better as it tries to maximize the value while the experiment runs. Common use cases are testing algorithms in an online setting, content and ads optimization and dynamic pricing.

The system is composed of an HTTP server exposing a REST API managing the life cycle of online experiments optimizing for binary rewards - such as clicks or conversions - where different variants of _something_ are exposed to a varying fraction of traffic depending on their historical performance.

While an experiment is running, an external service sends a request to get current best variant, collect some binary feedback (for instance a click) and then sends another request to update the state of the underlying policy using the observed reward. Eventually, the policy converges to the best performing variant, assuming a relatively stationary environment.

### Architecture

![architecture](assets/architecture.png "Architecture")

The HTTP server is implemented using [actix-web](https://actix.rs/) framework and the whole application follows an [actor model](https://en.wikipedia.org/wiki/Actor_model) using [actix](https://github.com/actix/actix?tab=readme-ov-file) framework to efficiently handle concurrency and isolate experiments.

In this paradigm, an actor is an independent entity that manages its own state and interacts with its environment - such as other actors - using asynchronous message passing only. Messages received are enqueued and processed sequentially within an actor, avoiding the complexities of lock mechanisms.

In our system, the HTTP server interacts with a **supervisor** actor that is responsible for managing the different experiments. Each experiment is an actor implementing some policy, itself handling the variant optimization. The supervisor either creates or deletes experiments, or simply dispatch a message the a running experiment. This allows to have low coupling between experiments and process requests for different requests in a non blocking way. 

The supervisor periodically checks the health of the experiments it manages and can, upon failure, restart them from a past cached state. Individual experiments periodically send their state to a cache actor, that is also persisted to disk for recovery in case of an application crash.

Finally, every request along with the response is processed in a middleware and sent to an **accountant** actor, responsible for tracking and interacting with some storage.

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes. See [deployment](#deployment) for notes on how to deploy the project on a live system.

### Prerequisites

What things you need to install the software and how to install them.

```
Give examples
```

### Installing

A step by step series of examples that tell you how to get a development env running.

Say what the step will be

```
Give the example
```

And repeat

```
until finished
```

End with an example of getting some data out of the system or using it for a little demo.

## API endpoints

The system currently exposes 12 routes:

| Request 	| Response 	| Description 	|
|---	|---	|---	|
| `GET v1/ping` 	|  	| send a ping request to the server 	|
| `GET v1/list` 	| `{"experiment_ids": [...]}` 	| return the id of all available experiments 	|
| `DELETE v1/clear` 	|  	| delete all experiments 	|
| `POST v1/create` 	| `{"experiment_id": ...}` 	| create a new experiment and return its unique id 	|
| `PUT v1/{experiment_id}/reset` 	|  	| reset the state of the experiment 	|
| `DELETE v1/{experiment_id}/delete` 	|  	| delete an experiment 	|
| `POST v1/{experiment_id}/add_arm` 	| `{"arm_id" : ...}` 	| create a new variant for a given experiment and return its id 	|
| `DELETE v1/{experiment_id}/delete_arm/{arm_id}` 	|  	| delete a given variant for a given experiment 	|
| `GET v1/{experiment_id}/draw` 	| `{"arm_id" : ...}` 	| get the current best performing variant of an experiment 	|
| `POST v1/{experiment_id}/update` 	|  	| update an experiment by sending a json with structure `{"ts": ...,"arm_id": ...,"reward":...}` 	|
| `POST v1/{experiment_id}/update_batch` 	|  	| send multiple updates for an experiment, with structure `{"updates": [...]}` 	|
| `GET v1/{experiment_id}/stats` 	| `{"arms": {"arm_id": {"pulls": ..., "mean_reward": ...}, ...}}` 	| return some basic stats for a given experiment 	|

## Roadmap

**Core**
- [ ] Implement storage for logs and its interactions with the accountant actor
- [ ] Improve cache persistence to allow for some historization
- [ ] Implement metrics collection system to monitor the service
- [ ] Provide authentication using JWT

**Policies**
- [ ] Optional epsilon decay (VDBE)
- [ ] UCB and its variants
- [ ] Thomson Sampling for binary rewards
- [ ] Decayed rewards to non stationary environments
- [ ] Contextual bandits

**UX**
- [ ] Routes to disable/enable variants and experiments
- [ ] Dashboard to manage and monitor experiments: variant selection rates, rewards, etc.