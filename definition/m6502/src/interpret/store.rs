#[macro_export]
macro_rules! store_m6502_addressing_modes {
    ($instruction:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr, [$($modes:ident),*]) => {{
        match $instruction.addressing_mode {
            $(
                Some(AddressingMode::$modes(argument)) => {
                    store_m6502_addressing_modes!(@handler $modes, argument, $register_store, $memory_translation_table, $assigned_address_space, $value)
                },
            )*
            _ => unreachable!(),
        }
    }};

    (@handler Absolute, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;

        let _ = $memory_translation_table
            .write($argument as usize, $assigned_address_space, std::array::from_ref(&value));
    }};

    (@handler XIndexedAbsolute, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;

        let _ = $memory_translation_table
            .read($argument as usize, $assigned_address_space, &mut [0]);

        let actual_address = $argument.wrapping_add($register_store.index_registers[0] as u16);
        let _ = $memory_translation_table
            .write(actual_address as usize, $assigned_address_space, std::array::from_ref(&value));
    }};

    (@handler YIndexedAbsolute, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;

        let _ = $memory_translation_table
            .read($argument as usize, $assigned_address_space, &mut [0]);

        let actual_address = $argument.wrapping_add($register_store.index_registers[1] as u16);

        let _ = $memory_translation_table
            .write(actual_address as usize, $assigned_address_space, std::array::from_ref(&value));
    }};

    (@handler ZeroPage, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;

        let _ = $memory_translation_table
            .write($argument as usize, $assigned_address_space, std::array::from_ref(&value));
    }};

    (@handler XIndexedZeroPage, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;

        let actual_address = $argument.wrapping_add($register_store.index_registers[0]);

        let _ = $memory_translation_table
            .write(actual_address as usize, $assigned_address_space, std::array::from_ref(&value));
    }};

    (@handler YIndexedZeroPage, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;
        let actual_address = $argument.wrapping_add($register_store.index_registers[1]);

        let _ = $memory_translation_table
            .write(actual_address as usize, $assigned_address_space, std::array::from_ref(&value));
    }};

    (@handler XIndexedZeroPageIndirect, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;

        let indirection_address = $argument.wrapping_add($register_store.index_registers[0]);
        let mut actual_address = [0; 2];

        let _ = $memory_translation_table
            .read(indirection_address as usize, $assigned_address_space, &mut actual_address);

        let actual_address = u16::from_le_bytes(actual_address);

        let _ = $memory_translation_table
            .write(actual_address as usize, $assigned_address_space, std::array::from_ref(&value));
    }};

    (@handler ZeroPageIndirectYIndexed, $argument:expr, $register_store:expr, $memory_translation_table:expr, $assigned_address_space:expr, $value:expr) => {{
        let value: u8 = $value;
        let mut indirection_address: [u8; 2] = [0; 2];

        let _ = $memory_translation_table
            .read($argument as usize, $assigned_address_space, &mut indirection_address);

        let indirection_address = u16::from_le_bytes(indirection_address);

        let indirection_address = (indirection_address as u16)
            .wrapping_add($register_store.index_registers[1] as u16);

        let _ = $memory_translation_table
            .write(indirection_address as usize, $assigned_address_space, std::array::from_ref(&value));
    }};
}
