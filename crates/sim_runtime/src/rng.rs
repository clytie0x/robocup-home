/// Owned xoshiro256** 1.0, seeded with splitmix64.
///
/// `next_u64` follows the public-domain xoshiro256** 1.0 function by Blackman and Vigna.
/// The seed expands with Vigna's 2015 fixed-increment splitmix64. This is not a cryptographic generator.
#[derive(Clone, Debug)]
pub struct Rng {
    state: [u64; 4],
}

fn splitmix_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut mixed = *state;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d049bb133111eb);
    mixed ^ (mixed >> 31)
}

impl Rng {
    pub(crate) fn from_seed(seed: u64) -> Self {
        let mut mix = seed;
        let state = [
            splitmix_next(&mut mix),
            splitmix_next(&mut mix),
            splitmix_next(&mut mix),
            splitmix_next(&mut mix),
        ];
        debug_assert!(state.iter().any(|word| *word != 0));
        Self { state }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        let state = &mut self.state;
        let result = state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let mixed = state[1] << 17;
        state[2] ^= state[0];
        state[3] ^= state[1];
        state[1] ^= state[2];
        state[0] ^= state[3];
        state[2] ^= mixed;
        state[3] = state[3].rotate_left(45);
        result
    }

    pub(crate) fn uniform(&mut self, span: u64) -> u64 {
        assert!(span > 0, "empty range");
        let limit = (u64::MAX / span) * span;
        loop {
            let draw = self.next_u64();
            if draw < limit {
                return draw % span;
            }
        }
    }

    pub(crate) fn uniform_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = u64::try_from(hi - lo).expect("ordered range") + 1;
        let offset = self.uniform(span);
        lo + i32::try_from(offset).expect("offset fits")
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;
    use crate::assume;

    #[test]
    fn seed_zero_matches_the_reference_first_draw() {
        let mut rng = Rng::from_seed(0);
        assert_eq!(rng.next_u64(), 0x99ec5f36cb75f2b4);
        assert_eq!(assume::PRNG, "xoshiro256**-1.0+splitmix64-2015");
    }

    #[test]
    fn different_seeds_differ_on_the_first_draw() {
        let mut left = Rng::from_seed(1);
        let mut right = Rng::from_seed(2);
        assert_ne!(left.next_u64(), right.next_u64());
    }
}
