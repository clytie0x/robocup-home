use std::num::NonZeroU32;

/// Entity id. Zero is not an id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(NonZeroU32);

impl Id {
    pub const fn new(raw: u32) -> Option<Self> {
        match NonZeroU32::new(raw) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::Id;

    #[test]
    fn zero_is_rejected_and_a_positive_id_round_trips() {
        assert_eq!(Id::new(0), None);
        assert_eq!(Id::new(4).map(Id::get), Some(4));
    }
}
