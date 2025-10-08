use crate::{
    cartridge::{
        NesCartridge,
        ines::{INesVersion, Mirroring, expansion_device::DefaultExpansionDevice},
    },
    gamepad::controller::NesControllerConfig,
    ppu::{
        BACKGROUND_PALETTE_BASE_ADDRESS, NAMETABLE_ADDRESSES,
        backend::SupportedGraphicsApiPpu,
        region::{Region, ntsc::Ntsc},
    },
};
pub use cartridge::ines::INes;
use cartridge::{NesCartridgeConfig, ines::TimingMode};
use multiemu::{
    machine::{MachineFactory, builder::MachineBuilder},
    memory::AddressSpaceId,
    platform::Platform,
};
use multiemu_definition_misc::memory::{
    mirror::MirrorMemoryConfig,
    standard::{StandardMemoryConfig, StandardMemoryInitialContents},
};
use multiemu_definition_mos6502::{Mos6502Config, Mos6502Kind};
use ppu::NesPpuConfig;
use rangemap::RangeInclusiveMap;
use std::marker::PhantomData;

mod apu;
mod cartridge;
mod gamepad;
mod ppu;

#[derive(Debug, Default)]
pub struct Nes;

impl<G: SupportedGraphicsApiPpu, P: Platform<GraphicsApi = G>> MachineFactory<P> for Nes {
    fn construct(&self, machine: MachineBuilder<P>) -> MachineBuilder<P> {
        let (machine, cpu_address_space) = machine.insert_address_space(16);
        let (machine, ppu_address_space) = machine.insert_address_space(14);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0x0000..=0x07ff,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0x0000..=0x07ff,
                    StandardMemoryInitialContents::Random,
                )]),
                sram: false,
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror-0",
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 0x0800..=0x0fff,
                source_address_space: cpu_address_space,
                destination_addresses: 0x0000..=0x07ff,
                destination_address_space: cpu_address_space,
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror-1",
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 0x1000..=0x17ff,
                source_address_space: cpu_address_space,
                destination_addresses: 0x0000..=0x07ff,
                destination_address_space: cpu_address_space,
            },
        );
        let (mut machine, _) = machine.insert_component(
            "workram-mirror-2",
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 0x1800..=0x1fff,
                source_address_space: cpu_address_space,
                destination_addresses: 0x0000..=0x07ff,
                destination_address_space: cpu_address_space,
            },
        );

        for (index, address) in (0x2000..=0x3fff).step_by(8).skip(1).enumerate() {
            machine = machine
                .insert_component(
                    &format!("ppu-register-mirror-{}", index),
                    MirrorMemoryConfig {
                        readable: true,
                        writable: true,
                        source_addresses: address..=address + 7,
                        source_address_space: cpu_address_space,
                        destination_addresses: 0x2000..=0x2007,
                        destination_address_space: cpu_address_space,
                    },
                )
                .0;
        }

        let rom = machine.user_specified_roms().unwrap().main.id;
        let (machine, cartridge) = machine.insert_component(
            "cartridge",
            NesCartridgeConfig {
                rom,
                cpu_address_space,
                ppu_address_space,
            },
        );

        let (machine, _) = machine.insert_component(
            "palette-ram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: BACKGROUND_PALETTE_BASE_ADDRESS
                    ..=BACKGROUND_PALETTE_BASE_ADDRESS + 0x1f,
                assigned_address_space: ppu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    BACKGROUND_PALETTE_BASE_ADDRESS..=BACKGROUND_PALETTE_BASE_ADDRESS + 0x1f,
                    StandardMemoryInitialContents::Random,
                )]),
                sram: false,
            },
        );

        let ines = machine
            .registry()
            .interact_by_path::<NesCartridge, _>(&cartridge, |cart| cart.rom())
            .unwrap();

        let machine = setup_ppu_nametables(machine, ppu_address_space, &ines);

        let default_expansion_device = match ines.version {
            INesVersion::V1 => None,
            INesVersion::V2 {
                default_expansion_device,
                ..
            } => default_expansion_device,
        }
        .unwrap_or(DefaultExpansionDevice::StandardControllers { swapped: false });

        let machine = match default_expansion_device {
            DefaultExpansionDevice::StandardControllers { .. } => {
                let (machine, _) = machine.insert_component(
                    "standard-nes-controller-0",
                    NesControllerConfig {
                        cpu_address_space,
                        controller_index: 0,
                    },
                );

                /*
                let (machine, _) = machine.insert_component(
                    "standard-nes-controller-1",
                    NesControllerConfig {
                        cpu_address_space,
                        controller_index: 1,
                    },
                );
                */

                machine
            }
            DefaultExpansionDevice::FourScore => todo!(),
            DefaultExpansionDevice::SimpleFamiconFourPlayerAdaptor => todo!(),
            DefaultExpansionDevice::VsSystem { address } => todo!(),
            DefaultExpansionDevice::VsZapper => todo!(),
            DefaultExpansionDevice::Zapper => todo!(),
            DefaultExpansionDevice::DualZapper => todo!(),
            DefaultExpansionDevice::BandaiHyperShotLightgun => todo!(),
            DefaultExpansionDevice::PowerPad { upside } => todo!(),
            DefaultExpansionDevice::FamilyTrainer { upside } => todo!(),
            DefaultExpansionDevice::ArkanoidVaus { kind } => todo!(),
            DefaultExpansionDevice::DualArkanoidVausFamicomPlusDataRecorder => todo!(),
            DefaultExpansionDevice::KonamiHyperShotController => todo!(),
            DefaultExpansionDevice::CoconutsPachinkoController => todo!(),
            DefaultExpansionDevice::ExcitingBoxingPunchingBag => todo!(),
            DefaultExpansionDevice::JissenMahjongController => todo!(),
            DefaultExpansionDevice::PartyTap => todo!(),
            DefaultExpansionDevice::OekaKidsTablet => todo!(),
            DefaultExpansionDevice::SunsoftBarcodeBattler => todo!(),
            DefaultExpansionDevice::MiraclePianoKeyboard => todo!(),
            DefaultExpansionDevice::PokkunMoguraa => todo!(),
            DefaultExpansionDevice::TopRider => todo!(),
            DefaultExpansionDevice::DoubleFisted => todo!(),
            DefaultExpansionDevice::Famicom3dSystem => todo!(),
            DefaultExpansionDevice::ドレミッコKeyboard => todo!(),
            DefaultExpansionDevice::Rob { mode } => todo!(),
            DefaultExpansionDevice::FamiconDataRecorder => todo!(),
            DefaultExpansionDevice::AsciiTurboFile => todo!(),
            DefaultExpansionDevice::IgsStorageBattleBox => todo!(),
            DefaultExpansionDevice::FamilyBasicKeyBoardPlusFamiconDataRecorder => todo!(),
            DefaultExpansionDevice::东达PECKeyboard => todo!(),
            DefaultExpansionDevice::普澤Bit79Keyboard => todo!(),
            DefaultExpansionDevice::小霸王Keyboard { mouse } => todo!(),
            DefaultExpansionDevice::SnesMouse => todo!(),
            DefaultExpansionDevice::Multicart => todo!(),
            DefaultExpansionDevice::SnesControllers => todo!(),
            DefaultExpansionDevice::RacerMateBicycle => todo!(),
            DefaultExpansionDevice::UForce => todo!(),
            DefaultExpansionDevice::CityPatrolmanLightgun => todo!(),
            DefaultExpansionDevice::SharpC1CassetteInterface => todo!(),
            DefaultExpansionDevice::ExcaliburSudokuPad => todo!(),
            DefaultExpansionDevice::ABLPinball => todo!(),
            DefaultExpansionDevice::GoldenNuggetCasino => todo!(),
            DefaultExpansionDevice::科达Keyboard => todo!(),
            DefaultExpansionDevice::PortTestController => todo!(),
            DefaultExpansionDevice::BandaiMultiGamePlayerGamepad => todo!(),
            DefaultExpansionDevice::VenomTvDanceMat => todo!(),
            DefaultExpansionDevice::LgTvRemoteControl => todo!(),
            DefaultExpansionDevice::FamicomNetworkController => todo!(),
            DefaultExpansionDevice::KingFishingController => todo!(),
            DefaultExpansionDevice::CroakyKaraokeController => todo!(),
            DefaultExpansionDevice::科王Keyboard => todo!(),
            DefaultExpansionDevice::泽诚Keyboard => todo!(),
        };

        

        /*
        let (machine, _) = machine.insert_component(
            "forced-execution-vector",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0xfffc..=0xfffd,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0xfffc..=0xfffd,
                    StandardMemoryInitialContents::Array(Cow::Owned(
                        (0xc000u16).to_le_bytes().to_vec(),
                    )),
                )]),
                sram: false,
            },
        );
        */

        match ines.timing_mode {
            TimingMode::Ntsc => {
                let processor_frequency = Ntsc::master_clock() / 12;

                let (machine, processor) = machine.insert_component(
                    "processor",
                    Mos6502Config {
                        frequency: processor_frequency,
                        assigned_address_space: cpu_address_space,
                        kind: Mos6502Kind::Ricoh2A0x,
                        broken_ror: false,
                    },
                );

                let (machine, _) = machine.insert_component(
                    "ppu",
                    NesPpuConfig::<'_, Ntsc> {
                        ppu_address_space,
                        cpu_address_space,
                        processor,
                        ines: &ines,
                        _phantom: PhantomData,
                    },
                );

                machine
            }
            TimingMode::Pal => todo!(),
            TimingMode::Multi => todo!(),
            TimingMode::Dendy => todo!(),
        }
    }
}

fn setup_ppu_nametables<P: Platform>(
    machine: MachineBuilder<P>,
    ppu_address_space: AddressSpaceId,
    ines: &INes,
) -> MachineBuilder<P> {
    match ines.mirroring {
        Mirroring::Vertical => {
            let (machine, _) = machine.insert_component(
                "nametable-0",
                StandardMemoryConfig {
                    assigned_address_space: ppu_address_space,
                    assigned_range: NAMETABLE_ADDRESSES[0].clone(),
                    readable: true,
                    writable: true,
                    initial_contents: RangeInclusiveMap::from_iter([(
                        NAMETABLE_ADDRESSES[0].clone(),
                        StandardMemoryInitialContents::Random,
                    )]),
                    sram: false,
                },
            );

            let (machine, _) = machine.insert_component(
                "nametable-1",
                StandardMemoryConfig {
                    assigned_address_space: ppu_address_space,
                    assigned_range: NAMETABLE_ADDRESSES[1].clone(),
                    readable: true,
                    writable: true,
                    initial_contents: RangeInclusiveMap::from_iter([(
                        NAMETABLE_ADDRESSES[1].clone(),
                        StandardMemoryInitialContents::Random,
                    )]),
                    sram: false,
                },
            );

            let (machine, _) = machine.insert_component(
                "nametable-2",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    source_addresses: NAMETABLE_ADDRESSES[2].clone(),
                    source_address_space: ppu_address_space,
                    destination_addresses: NAMETABLE_ADDRESSES[0].clone(),
                    destination_address_space: ppu_address_space,
                },
            );

            let (machine, _) = machine.insert_component(
                "nametable-3",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    source_addresses: NAMETABLE_ADDRESSES[3].clone(),
                    source_address_space: ppu_address_space,
                    destination_addresses: NAMETABLE_ADDRESSES[1].clone(),
                    destination_address_space: ppu_address_space,
                },
            );

            machine
        }
        Mirroring::Horizontal => {
            let (machine, _) = machine.insert_component(
                "nametable-0",
                StandardMemoryConfig {
                    assigned_address_space: ppu_address_space,
                    assigned_range: NAMETABLE_ADDRESSES[0].clone(),
                    readable: true,
                    writable: true,
                    initial_contents: RangeInclusiveMap::from_iter([(
                        NAMETABLE_ADDRESSES[0].clone(),
                        StandardMemoryInitialContents::Random,
                    )]),
                    sram: false,
                },
            );

            let (machine, _) = machine.insert_component(
                "nametable-1",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    source_addresses: NAMETABLE_ADDRESSES[1].clone(),
                    source_address_space: ppu_address_space,
                    destination_addresses: NAMETABLE_ADDRESSES[0].clone(),
                    destination_address_space: ppu_address_space,
                },
            );

            let (machine, _) = machine.insert_component(
                "nametable-2",
                StandardMemoryConfig {
                    assigned_address_space: ppu_address_space,
                    assigned_range: NAMETABLE_ADDRESSES[2].clone(),
                    readable: true,
                    writable: true,
                    initial_contents: RangeInclusiveMap::from_iter([(
                        NAMETABLE_ADDRESSES[2].clone(),
                        StandardMemoryInitialContents::Random,
                    )]),
                    sram: false,
                },
            );

            let (machine, _) = machine.insert_component(
                "nametable-3",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    source_addresses: NAMETABLE_ADDRESSES[3].clone(),
                    source_address_space: ppu_address_space,
                    destination_addresses: NAMETABLE_ADDRESSES[2].clone(),
                    destination_address_space: ppu_address_space,
                },
            );

            machine
        }
    }
}
