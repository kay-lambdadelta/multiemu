use std::i8;

use crate::{ExecutionMode, FlagRegister, tests::mos6502::instruction_test_boilerplate};
use bitvec::{order::Lsb0, view::BitView};
use multiemu_runtime::utils::set_main_thread;

#[test]
pub fn adc_immediate() {
    set_main_thread();

    for value in 0x00..=0xff {
        let (machine, cpu, cpu_address_space) = instruction_test_boilerplate();

        // Enable carry
        cpu.interact(|component| {
            let mut state_guard = component.state.lock().unwrap();

            state_guard.flags.carry = true;
            state_guard.execution_mode = Some(ExecutionMode::FetchAndDecode);
        })
        .unwrap();

        machine
            .memory_access_table
            .write(0x0000, cpu_address_space, &[0x69, value])
            .unwrap();

        // Should be done in 2 cycles
        machine.scheduler.lock().unwrap().run_for_cycles(2);

        cpu.interact(|component| {
            let state_guard = component.state.lock().unwrap();
            let modified_value = value.wrapping_add(1);

            assert_eq!(state_guard.a, modified_value);
            assert_eq!(
                state_guard.flags,
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
            assert_eq!(state_guard.program, 0x2);
        })
        .unwrap();
    }
}

#[test]
pub fn adc_absolute() {
    set_main_thread();

    for value in 0x00..=0xff {
        let (machine, cpu, cpu_address_space) = instruction_test_boilerplate();

        machine
            .memory_access_table
            .write_le_value::<u8>(0x3, cpu_address_space, value)
            .unwrap();

        // Enable carry
        cpu.interact(|component| {
            let mut state_guard = component.state.lock().unwrap();

            state_guard.flags.carry = true;
            state_guard.execution_mode = Some(ExecutionMode::FetchAndDecode);
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

        machine.scheduler.lock().unwrap().run_for_cycles(4);

        cpu.interact(|component| {
            let state_guard = component.state.lock().unwrap();
            let modified_value = value.wrapping_add(1);

            assert_eq!(state_guard.a, modified_value);
            assert_eq!(
                state_guard.flags,
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
            assert_eq!(state_guard.program, 0x3);
        })
        .unwrap();
    }
}
