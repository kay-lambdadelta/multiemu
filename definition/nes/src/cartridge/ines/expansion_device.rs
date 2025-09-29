#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]

pub enum 小霸王KeyboardMouse {
    Mouse3x8,
    Mouse24 { address: u16 },
    Ps2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]

pub enum RobMode {
    Gyro,
    Stackup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArkanoidVausKind {
    Nes,
    Famicom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefaultExpansionDevice {
    StandardControllers {
        swapped: bool,
    },
    FourScore,
    SimpleFamiconFourPlayerAdaptor,
    VsSystem {
        address: u16,
    },
    VsZapper,
    Zapper,
    DualZapper,
    BandaiHyperShotLightgun,
    PowerPad {
        upside: bool,
    },
    FamilyTrainer {
        upside: bool,
    },
    ArkanoidVaus {
        kind: ArkanoidVausKind,
    },
    DualArkanoidVausFamicomPlusDataRecorder,
    KonamiHyperShotController,
    CoconutsPachinkoController,
    ExcitingBoxingPunchingBag,
    JissenMahjongController,
    PartyTap,
    OekaKidsTablet,
    SunsoftBarcodeBattler,
    MiraclePianoKeyboard,
    PokkunMoguraa,
    TopRider,
    DoubleFisted,
    Famicom3dSystem,
    ドレミッコKeyboard,
    Rob {
        mode: RobMode,
    },
    FamiconDataRecorder,
    AsciiTurboFile,
    IgsStorageBattleBox,
    FamilyBasicKeyBoardPlusFamiconDataRecorder,
    东达PECKeyboard,
    普澤Bit79Keyboard,
    小霸王Keyboard {
        mouse: Option<小霸王KeyboardMouse>,
    },
    SnesMouse,
    Multicart,
    SnesControllers,
    RacerMateBicycle,
    UForce,
    CityPatrolmanLightgun,
    SharpC1CassetteInterface,
    ExcaliburSudokuPad,
    ABLPinball,
    GoldenNuggetCasino,
    科达Keyboard,
    PortTestController,
    BandaiMultiGamePlayerGamepad,
    VenomTvDanceMat,
    LgTvRemoteControl,
    FamicomNetworkController,
    KingFishingController,
    CroakyKaraokeController,
    科王Keyboard,
    泽诚Keyboard,
}

impl DefaultExpansionDevice {
    pub fn new(id: u8) -> Option<Self> {
        match id {
            0x00 => None,
            0x01 => Some(Self::StandardControllers { swapped: false }),
            0x02 => Some(Self::FourScore),
            0x03 => Some(Self::SimpleFamiconFourPlayerAdaptor),
            0x04 => Some(Self::VsSystem { address: 0x4016 }),
            0x05 => Some(Self::VsSystem { address: 0x4017 }),
            // Reserved
            0x06 => None,
            0x07 => Some(Self::VsZapper),
            0x08 => Some(Self::Zapper),
            0x09 => Some(Self::DualZapper),
            0x0a => Some(Self::BandaiHyperShotLightgun),
            0x0b => Some(Self::PowerPad { upside: true }),
            0x0c => Some(Self::PowerPad { upside: false }),
            0x0d => Some(Self::FamilyTrainer { upside: true }),
            0x0e => Some(Self::FamilyTrainer { upside: false }),
            0x0f => Some(Self::ArkanoidVaus {
                kind: ArkanoidVausKind::Nes,
            }),
            0x10 => Some(Self::ArkanoidVaus {
                kind: ArkanoidVausKind::Famicom,
            }),
            0x11 => Some(Self::DualArkanoidVausFamicomPlusDataRecorder),
            0x12 => Some(Self::KonamiHyperShotController),
            0x13 => Some(Self::CoconutsPachinkoController),
            0x14 => Some(Self::ExcitingBoxingPunchingBag),
            0x15 => Some(Self::JissenMahjongController),
            0x16 => Some(Self::PartyTap),
            0x17 => Some(Self::OekaKidsTablet),
            0x18 => Some(Self::SunsoftBarcodeBattler),
            0x19 => Some(Self::MiraclePianoKeyboard),
            0x1a => Some(Self::PokkunMoguraa),
            0x1b => Some(Self::TopRider),
            0x1c => Some(Self::DoubleFisted),
            0x1d => Some(Self::Famicom3dSystem),
            0x1e => Some(Self::ドレミッコKeyboard),
            0x1f => Some(Self::Rob {
                mode: RobMode::Gyro,
            }),
            0x20 => Some(Self::FamiconDataRecorder),
            0x21 => Some(Self::AsciiTurboFile),
            0x22 => Some(Self::IgsStorageBattleBox),
            0x23 => Some(Self::FamilyBasicKeyBoardPlusFamiconDataRecorder),
            0x24 => Some(Self::东达PECKeyboard),
            0x25 => Some(Self::普澤Bit79Keyboard),
            0x26 => Some(Self::小霸王Keyboard { mouse: None }),
            0x27 => Some(Self::小霸王Keyboard {
                mouse: Some(小霸王KeyboardMouse::Mouse3x8),
            }),
            0x28 => Some(Self::小霸王Keyboard {
                mouse: Some(小霸王KeyboardMouse::Mouse24 { address: 0x4016 }),
            }),
            0x29 => Some(Self::SnesMouse),
            0x2a => Some(Self::Multicart),
            0x2b => Some(Self::SnesControllers),
            0x2c => Some(Self::RacerMateBicycle),
            0x2d => Some(Self::UForce),
            0x2e => Some(Self::Rob {
                mode: RobMode::Stackup,
            }),
            0x2f => Some(Self::CityPatrolmanLightgun),
            0x30 => Some(Self::SharpC1CassetteInterface),
            0x31 => Some(Self::StandardControllers { swapped: true }),
            0x32 => Some(Self::ExcaliburSudokuPad),
            0x33 => Some(Self::ABLPinball),
            0x34 => Some(Self::GoldenNuggetCasino),
            0x35 => Some(Self::科达Keyboard),
            0x36 => Some(Self::小霸王Keyboard {
                mouse: Some(小霸王KeyboardMouse::Mouse24 { address: 0x4017 }),
            }),
            0x37 => Some(Self::PortTestController),
            0x38 => Some(Self::BandaiMultiGamePlayerGamepad),
            0x39 => Some(Self::VenomTvDanceMat),
            0x3a => Some(Self::LgTvRemoteControl),
            0x3b => Some(Self::FamicomNetworkController),
            0x3c => Some(Self::KingFishingController),
            0x3d => Some(Self::CroakyKaraokeController),
            0x3e => Some(Self::科王Keyboard),
            0x3f => Some(Self::泽诚Keyboard),
            0x40 => Some(Self::小霸王Keyboard {
                mouse: Some(小霸王KeyboardMouse::Ps2),
            }),
            _ => None,
        }
    }
}
