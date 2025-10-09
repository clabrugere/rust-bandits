use rand::{rngs::SmallRng, SeedableRng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MaybeSeededRng {
    pub seed: Option<u64>,
    #[serde(skip)]
    #[serde(default = "default_rng")]
    rng: SmallRng,
}

fn default_rng() -> SmallRng {
    SmallRng::from_os_rng()
}

impl MaybeSeededRng {
    pub fn new(seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(seed) => SmallRng::seed_from_u64(seed),
            None => SmallRng::from_os_rng(),
        };
        Self { seed, rng }
    }

    pub fn get_rng(&mut self) -> &mut SmallRng {
        &mut self.rng
    }
}
