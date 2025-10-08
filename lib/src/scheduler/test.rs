use num::rational::Ratio;

use crate::{
    component::{Component, ComponentConfig},
    machine::{Machine, builder::ComponentBuilder},
    platform::Platform,
};
use std::{num::NonZero, time::Duration};

#[test]
fn basic_operation() {
    #[derive(Debug)]
    struct Counter {
        counter: u32,
    }

    impl Component for Counter {}

    #[derive(Debug)]
    struct CounterConfig {
        frequency: Ratio<u32>,
    }

    impl<P: Platform> ComponentConfig<P> for CounterConfig {
        type Component = Counter;

        fn build_component(
            self,
            component_builder: ComponentBuilder<P, Self::Component>,
        ) -> Result<Self::Component, crate::component::BuildError> {
            component_builder.insert_task_mut(
                "task",
                self.frequency,
                |component: &mut Counter, slice: NonZero<u32>| {
                    component.counter += slice.get();
                },
            );

            Ok(Counter { counter: 0 })
        }
    }

    let (machine, counter1000) = Machine::build_test_minimal().insert_component(
        "counter1000",
        CounterConfig {
            frequency: Ratio::from_integer(1000),
        },
    );
    let (machine, counter10000) = machine.insert_component(
        "counter10000",
        CounterConfig {
            frequency: Ratio::from_integer(10000),
        },
    );

    let mut machine = machine.build(Default::default(), false);

    machine
        .scheduler_state
        .as_mut()
        .unwrap()
        .run(Duration::from_secs(1));

    machine
        .component_registry
        .interact_by_path::<Counter, _>(&counter1000, |component| {
            assert_eq!(component.counter, 1000);
        })
        .unwrap();

    machine
        .component_registry
        .interact_by_path::<Counter, _>(&counter10000, |component| {
            assert_eq!(component.counter, 10000);
        })
        .unwrap();
}
