use crate::{
    component::{Component, ComponentConfig, SynchronizationContext},
    machine::{
        Machine,
        builder::{ComponentBuilder, SchedulerParticipation},
    },
    platform::Platform,
    scheduler::Period,
};
use num::FromPrimitive;
use std::time::Duration;

#[test]
fn basic_operation() {
    #[derive(Debug)]
    struct TestComponent {
        counter: u32,
    }

    impl Component for TestComponent {
        fn synchronize(&mut self, mut context: SynchronizationContext) {
            for _ in context.allocate(Period::ONE / 1000, None) {
                self.counter += 1;
            }
        }

        fn needs_work(&self, delta: Period) -> bool {
            delta >= Period::ONE / 1000
        }
    }

    #[derive(Debug)]
    struct TestComponentConfig {
        scheduler_participation: SchedulerParticipation,
    }

    impl<P: Platform> ComponentConfig<P> for TestComponentConfig {
        type Component = TestComponent;

        fn build_component(
            self,
            component_builder: ComponentBuilder<P, Self::Component>,
        ) -> Result<Self::Component, Box<dyn std::error::Error>> {
            component_builder.set_scheduler_participation(self.scheduler_participation);

            Ok(TestComponent { counter: 0 })
        }
    }

    let (machine, scheduler_driven) = Machine::build_test_minimal().insert_component(
        "test_scheduler_driven",
        TestComponentConfig {
            scheduler_participation: SchedulerParticipation::SchedulerDriven,
        },
    );

    let (machine, on_demand) = machine.insert_component(
        "test_on_demand",
        TestComponentConfig {
            scheduler_participation: SchedulerParticipation::OnDemand,
        },
    );

    let machine = machine.build(());

    machine.run_duration(Duration::from_secs(1));
    machine
        .registry
        .interact_without_synchronization::<TestComponent, _>(&scheduler_driven, |component| {
            assert_eq!(component.counter, 1000);
        });
    machine
        .registry
        .interact::<TestComponent, _>(&scheduler_driven, machine.now(), |component| {
            assert_eq!(component.counter, 1000);
        });
    machine
        .registry
        .interact_without_synchronization::<TestComponent, _>(&on_demand, |component| {
            assert_eq!(component.counter, 0);
        });
    machine
        .registry
        .interact::<TestComponent, _>(&on_demand, machine.now(), |component| {
            assert_eq!(component.counter, 1000);
        });
}

#[test]
fn basic_events() {
    #[derive(Debug)]
    struct TestComponent {
        counter: u32,
        event_counter: u32,
    }

    impl Component for TestComponent {
        fn synchronize(&mut self, mut context: SynchronizationContext) {
            for _ in context.allocate(Period::ONE / 1000, None) {
                assert_eq!(self.counter, self.event_counter);

                self.counter += 1;
            }
        }

        fn needs_work(&self, delta: Period) -> bool {
            delta >= Period::ONE / 1000
        }
    }

    #[derive(Debug)]
    struct TestComponentConfig {
        scheduler_participation: SchedulerParticipation,
    }

    impl<P: Platform> ComponentConfig<P> for TestComponentConfig {
        type Component = TestComponent;

        fn build_component(
            self,
            component_builder: ComponentBuilder<P, Self::Component>,
        ) -> Result<Self::Component, Box<dyn std::error::Error>> {
            let frequency = Period::from_u64(1000).unwrap();

            component_builder
                .set_scheduler_participation(self.scheduler_participation)
                .schedule_repeating_event(frequency.recip(), frequency, |component, _| {
                    component.event_counter += 1;
                });

            Ok(TestComponent {
                counter: 0,
                event_counter: 0,
            })
        }
    }

    let (machine, scheduler_driven) = Machine::build_test_minimal().insert_component(
        "test_scheduler_driven",
        TestComponentConfig {
            scheduler_participation: SchedulerParticipation::SchedulerDriven,
        },
    );

    let (machine, on_demand) = machine.insert_component(
        "test_on_demand",
        TestComponentConfig {
            scheduler_participation: SchedulerParticipation::OnDemand,
        },
    );

    let machine = machine.build(());

    machine.run_duration(Duration::from_secs(1));
    machine
        .registry
        .interact_without_synchronization::<TestComponent, _>(&scheduler_driven, |component| {
            assert_eq!(component.counter, 1000);
        });
    machine
        .registry
        .interact::<TestComponent, _>(&scheduler_driven, machine.now(), |component| {
            assert_eq!(component.counter, 1000);
            assert_eq!(component.event_counter, 1000);
        });
    machine
        .registry
        .interact_without_synchronization::<TestComponent, _>(&on_demand, |component| {
            assert_eq!(component.counter, 1000);
        });
    machine
        .registry
        .interact::<TestComponent, _>(&on_demand, machine.now(), |component| {
            assert_eq!(component.counter, 1000);
            assert_eq!(component.event_counter, 1000);
        });
}
