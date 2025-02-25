#[derive(Debug)]
pub struct Mapper {
    mapper: u16,
    submapper: u16,
}

impl Mapper {
    pub fn new(mapper: u16, submapper: u16) -> Self {
        #[allow(clippy::zero_prefixed_literal)]
        let actual_mapper = match mapper {
            039 => Some(241),
            101 => Some(087),
            102 => Some(284),
            129 => Some(058),
            130 => Some(331),
            160 => Some(090),
            161 => Some(001),
            _ => None,
        };

        if let Some(actual_mapper) = actual_mapper {
            tracing::info!("Overriding mapper {} with {}", mapper, actual_mapper);
        }

        Self {
            mapper: actual_mapper.unwrap_or(mapper),
            submapper,
        }
    }
}
