use multiemu_audio::Sample;
use nalgebra::SVector;

pub trait AudioCallback<S: Sample>: Send + Sync + 'static {
    fn generate_audio(&self) -> Box<dyn Iterator<Item = SVector<S, 2>> + '_>;
}
