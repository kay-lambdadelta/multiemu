use bitvec::{order::Lsb0, view::BitView};
use fluxemu_runtime::scheduler::Period;

use crate::{ExecutionStep, FlagRegister, Mos6502, tests::mos6502::instruction_test_boilerplate};

#[test]
pub fn adc_immediate() {
    for value in 0x00..=0xff {
        let (machine, cpu, address_space) = instruction_test_boilerplate();
        let address_space = machine.address_spaces(address_space).unwrap();

        // Enable carry
        machine
            .interact_mut::<Mos6502, _>(&cpu, |component| {
                component.state.flags.carry = true;
                component.state.execution_queue.clear();
                component
                    .state
                    .execution_queue
                    .push_back(ExecutionStep::FetchAndDecode);
            })
            .unwrap();

        address_space
            .write(0x0000, machine.now(), None, &[0x69, value])
            .unwrap();

        // Should be done in 2 cycles
        machine.run(Period::from_num(2));

        machine
            .interact::<Mos6502, _>(&cpu, |component| {
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
        let (machine, cpu, address_space) = instruction_test_boilerplate();
        let address_space = machine.address_spaces(address_space).unwrap();

        address_space
            .write_le_value::<u8>(0x3, machine.now(), None, value)
            .unwrap();

        // Enable carry
        machine
            .interact_mut::<Mos6502, _>(&cpu, |component| {
                component.state.flags.carry = true;
                component.state.execution_queue.clear();
                component
                    .state
                    .execution_queue
                    .push_back(ExecutionStep::FetchAndDecode);
            })
            .unwrap();

        // ADC 0x0003
        address_space
            .write_le_value::<u8>(0x0000, machine.now(), None, 0x6d)
            .unwrap();
        address_space
            .write_le_value::<u16>(0x0001, machine.now(), None, 0x3)
            .unwrap();

        machine.run(Period::from_num(4));

        machine
            .interact::<Mos6502, _>(&cpu, |component| {
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
