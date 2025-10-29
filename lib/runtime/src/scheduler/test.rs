use crate::{
    component::{Component, ComponentConfig},
    machine::{Machine, builder::ComponentBuilder},
    platform::Platform,
    scheduler::TaskType,
};
use num::rational::Ratio;
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
        ) -> Result<Self::Component, Box<dyn std::error::Error>> {
            component_builder.insert_task(
                "task",
                self.frequency,
                TaskType::Direct,
                |component: &mut Counter, slice: NonZero<u32>| {
                    component.counter += slice.get();
                },
            );

            Ok(Counter { counter: 0 })
        }
    }

    let machine = Machine::build_test_minimal();
    let (machine, counter1000) = machine.insert_component(
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

    let mut machine = machine.build(());

    machine
        .scheduler
        .as_mut()
        .unwrap()
        .run(Duration::from_secs(1));

    machine
        .component_registry
        .interact::<Counter, _>(&counter1000, |component| {
            assert_eq!(component.counter, 1000);
        })
        .unwrap();

    machine
        .component_registry
        .interact::<Counter, _>(&counter10000, |component| {
            assert_eq!(component.counter, 10000);
        })
        .unwrap();
}

#[test]
fn basic_operation_lazy() {
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
        ) -> Result<Self::Component, Box<dyn std::error::Error>> {
            component_builder.insert_task(
                "task",
                self.frequency,
                TaskType::Lazy,
                |component: &mut Counter, slice: NonZero<u32>| {
                    component.counter += slice.get();
                },
            );

            Ok(Counter { counter: 0 })
        }
    }

    let machine = Machine::build_test_minimal();
    let (machine, counter1000) = machine.insert_component(
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

    let mut machine = machine.build(());

    machine
        .scheduler
        .as_mut()
        .unwrap()
        .run(Duration::from_secs(1));

    machine
        .component_registry
        .interact::<Counter, _>(&counter1000, |component| {
            assert_eq!(component.counter, 1000);
        })
        .unwrap();

    machine
        .component_registry
        .interact::<Counter, _>(&counter10000, |component| {
            assert_eq!(component.counter, 10000);
        })
        .unwrap();
}
