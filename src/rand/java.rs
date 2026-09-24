use std::num::Wrapping;

#[derive(Clone, Debug)]
pub struct JavaRand(Wrapping<i64>);

const MULTIPLIER: Wrapping<i64> = Wrapping(0x5DEECE66D);
const ADDEND: Wrapping<i64> = Wrapping(0xB);
const MASK: Wrapping<i64> = Wrapping((1i64 << 48) - 1);

impl JavaRand {
    pub fn new(seed: i64) -> Self {
        let s = (Wrapping(seed) ^ MULTIPLIER) & MASK;
        JavaRand(s)
    }

    pub fn get_seed(&self) -> i64 {
        self.0.0
    }

    pub fn set_seed(&mut self, seed: i64) {
        *self = JavaRand::new(seed);
    }

    pub fn bits<const N: usize>(&mut self) -> i64 {
        const {
            assert!(N <= 48, "cannot return more than 48 bits");
        }

        self.0 = (self.0 * MULTIPLIER + ADDEND) & MASK;
        self.0.0 >> (48 - N)
    }

    pub fn random<T>(&mut self) -> T
    where
        T: JavaRandType,
        <T as JavaRandType>::Extra: Default,
    {
        T::random(self)
    }

    pub fn random_with<T>(&mut self, extra: T::Extra) -> T
    where
        T: JavaRandType,
    {
        T::random_with(self, extra)
    }
}

pub trait JavaRandType: Sized {
    type Extra;

    fn random_with(rand: &mut JavaRand, p: Self::Extra) -> Self;

    fn random(rand: &mut JavaRand) -> Self
    where
        Self::Extra: Default,
    {
        Self::random_with(rand, Self::Extra::default())
    }
}
