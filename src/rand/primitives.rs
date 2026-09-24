use crate::rand::java::{JavaRand, JavaRandType};
use std::num::NonZeroU32;

impl JavaRandType for bool {
    type Extra = ();

    fn random_with(rand: &mut JavaRand, _: Self::Extra) -> Self {
        rand.bits::<1>() != 0
    }
}

#[derive(Default)]
pub struct Bound(Option<NonZeroU32>);

impl Bound {
    pub fn new(n: i32) -> Self {
        Bound(u32::try_from(n).ok().and_then(NonZeroU32::new))
    }
}

impl JavaRandType for i32 {
    type Extra = Bound;

    fn random_with(rand: &mut JavaRand, extra: Self::Extra) -> Self {
        match extra.0 {
            None => rand.bits::<32>() as i32,
            Some(b) => {
                let bound = b.get();
                match bound.is_power_of_two() {
                    true => ((bound as i64 * rand.bits::<31>()) >> 31) as i32,
                    false => loop {
                        let bound = bound as i32;
                        let bits = rand.bits::<31>() as i32;
                        let val = bits % bound;
                        if bits - val + (bound - 1) >= 0 {
                            break val;
                        }
                    },
                }
            }
        }
    }
}

impl JavaRandType for i64 {
    type Extra = ();

    fn random_with(rand: &mut JavaRand, _: Self::Extra) -> Self {
        let hi = rand.random::<i32>() as i64;
        let lo = rand.random::<i32>() as i64;
        (hi << 32).wrapping_add(lo)
    }
}

impl JavaRandType for f32 {
    type Extra = ();

    fn random_with(rand: &mut JavaRand, _: Self::Extra) -> Self {
        rand.bits::<24>() as f32 / ((1i32 << 24) as f32)
    }
}

impl JavaRandType for f64 {
    type Extra = ();

    fn random_with(rand: &mut JavaRand, _: Self::Extra) -> Self {
        let hi = rand.bits::<26>();
        let lo = rand.bits::<27>();
        (((hi << 27) | lo) as f64) / ((1i64 << 53) as f64)
    }
}
