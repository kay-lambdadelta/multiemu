use crate::{frontend::MaybeMachine, platform::Platform};
use num::rational::Ratio;
use std::{fmt::Debug, sync::Arc};

pub trait AudioContext<P: Platform>: Debug {
    fn new(maybe_machine: Arc<MaybeMachine<P>>) -> Self;
    fn sample_rate(&self) -> Ratio<u32>;
    fn pause(&self);
    fn play(&self);
}
