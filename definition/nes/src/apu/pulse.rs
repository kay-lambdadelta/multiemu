use crate::apu::Apu;
use multiemu_runtime::memory::Address;

#[derive(Debug, Default, Clone, Copy)]
pub struct Sweep {
    pub enabled: bool,
    pub period: u8,
    pub negate: bool,
    pub shift: u8,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PulseChannel {
    pub duty: u8,
    pub length_counter_halt: bool,
    pub length_counter_load: u8,
    pub constant_volume: bool,
    pub volume: u8,
    pub timer: u16,
    pub sweep: Sweep,
    pub enabled: bool,
}

impl Apu {
    pub(super) fn pulse_write(&mut self, index: u8, position: Address, byte: u8) {
        let pulse_channel = &mut self.pulse_channels[index as usize];

        match position {
            0 => {
                let duty = (0b1100_0000 & byte) >> 6;
                let length_counter_halt = (0b0010_0000 & byte) != 0;
                let constant_volume = (0b0001_0000 & byte) != 0;
                let volume = 0b0000_1111 & byte;

                pulse_channel.duty = duty;
                pulse_channel.length_counter_halt = length_counter_halt;
                pulse_channel.constant_volume = constant_volume;
                pulse_channel.volume = volume;
            }
            1 => {
                let enabled = (0b1000_0000 & byte) != 0;
                let period = (0b0111_0000 & byte) >> 4;
                let negate = (0b0000_1000 & byte) != 0;
                let shift = 0b0000_0111 & byte;

                pulse_channel.sweep = Sweep {
                    enabled,
                    period,
                    negate,
                    shift,
                };
            }
            2 => {
                let mut timer_contents = pulse_channel.timer.to_le_bytes();
                timer_contents[0] = byte;
                pulse_channel.timer = u16::from_le_bytes(timer_contents);
            }
            3 => {
                let mut timer_contents = pulse_channel.timer.to_le_bytes();
                timer_contents[1] = byte & 0b0000_0111;
                pulse_channel.timer = u16::from_le_bytes(timer_contents);

                let length_counter_load = (byte & 0b1111_1000) >> 3;
                pulse_channel.length_counter_load = length_counter_load;
            }
            _ => {
                unreachable!()
            }
        }
    }
}
