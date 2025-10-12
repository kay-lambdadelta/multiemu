use crate::{ExecutionStep, FlagRegister, Mos6502, tests::mos6502::instruction_test_boilerplate};
use bitvec::{order::Lsb0, view::BitView};

#[test]
pub fn adc_immediate() {
    for value in 0x00..=0xff {
        let (mut machine, cpu, cpu_address_space) = instruction_test_boilerplate();
        let component_id = machine.component_registry.get_id(&cpu).unwrap();

        // Enable carry
        machine
            .component_registry
            .interact_mut::<Mos6502, _>(component_id, |component| {
                component.state.flags.carry = true;
                component.state.execution_queue.clear();
                component
                    .state
                    .execution_queue
                    .push_back(ExecutionStep::FetchAndDecode);
            })
            .unwrap();

        machine
            .memory_access_table
            .write(0x0000, cpu_address_space, &[0x69, value])
            .unwrap();

        // Should be done in 2 cycles
        machine.scheduler_state.as_mut().unwrap().run_for_cycles(2);

        machine
            .component_registry
            .interact::<Mos6502, _>(component_id, |component| {
                let modified_value = value.wrapping_add(1);

                assert_eq!(component.state.a, modified_value);
                assert_eq!(
                    component.state.flags,
                    FlagRegister {
                        negative: modified_value.view_bits::<Lsb0>()[7],
                        overflow: bytemuck::cast::<_, i8>(value).checked_add(1).is_none(),
                        undocumented: false,
                        break_: false,
                        decimal: false,
                        interrupt_disable: false,
                        zero: value.wrapping_add(1) == 0,
                        carry: value.checked_add(1).is_none()
                    }
                );
                assert_eq!(component.state.program, 0x2);
            })
            .unwrap();
    }
}

#[test]
pub fn adc_absolute() {
    for value in 0x00..=0xff {
        let (mut machine, cpu, cpu_address_space) = instruction_test_boilerplate();
        let component_id = machine.component_registry.get_id(&cpu).unwrap();

        machine
            .memory_access_table
            .write_le_value::<u8>(0x3, cpu_address_space, value)
            .unwrap();

        // Enable carry
        machine
            .component_registry
            .interact_mut::<Mos6502, _>(component_id, |component| {
                component.state.flags.carry = true;
                component.state.execution_queue.clear();
                component
                    .state
                    .execution_queue
                    .push_back(ExecutionStep::FetchAndDecode);
            })
            .unwrap();

        // ADC 0x0003
        machine
            .memory_access_table
            .write_le_value::<u8>(0x0000, cpu_address_space, 0x6d)
            .unwrap();

        machine
            .memory_access_table
            .write_le_value::<u16>(0x0001, cpu_address_space, 0x3)
            .unwrap();

        machine.scheduler_state.as_mut().unwrap().run_for_cycles(4);

        machine
            .component_registry
            .interact::<Mos6502, _>(component_id, |component| {
                let modified_value = value.wrapping_add(1);

                assert_eq!(component.state.a, modified_value);
                assert_eq!(
                    component.state.flags,
                    FlagRegister {
                        negative: modified_value.view_bits::<Lsb0>()[7],
                        overflow: bytemuck::cast::<_, i8>(value).checked_add(1).is_none(),
                        undocumented: false,
                        break_: false,
                        decimal: false,
                        interrupt_disable: false,
                        zero: value.wrapping_add(1) == 0,
                        carry: value.checked_add(1).is_none()
                    }
                );
                assert_eq!(component.state.program, 0x3);
            })
            .unwrap();
    }
}
