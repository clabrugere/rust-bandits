use rand::{rngs::SmallRng, SeedableRng};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize)]
pub struct MaybeSeededRng {
    seed: Option<u64>,
    #[serde(skip)]
    rng: SmallRng,
}

impl MaybeSeededRng {
    pub fn new(seed: Option<u64>) -> Self {
        let rng = if let Some(seed) = seed {
            SmallRng::seed_from_u64(seed)
        } else {
            SmallRng::from_entropy()
        };

        Self { seed, rng }
    }

    pub fn get_rng(&mut self) -> &mut SmallRng {
        &mut self.rng
    }
}

impl<'de> Deserialize<'de> for MaybeSeededRng {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seed = Deserialize::deserialize(deserializer)?;
        Ok(Self::new(seed))
    }
}
