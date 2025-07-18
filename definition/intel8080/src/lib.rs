use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentRef},
    platform::Platform,
};
use multiemu_save::ComponentSave;

// mod decode;
// mod instruction;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Intel8080Kind {
    #[default]
    Intel8080,
    Zilog80,
    SharpLr35902,
}

#[derive(Debug)]
pub struct Intel8080 {
    config: Intel8080Config,
}

impl Component for Intel8080 {}

#[derive(Default, Debug)]
pub struct Intel8080Config {
    pub kind: Intel8080Kind,
}

impl Intel8080Config {
    pub fn lr35902() -> Self {
        Self {
            kind: Intel8080Kind::SharpLr35902,
        }
    }

    pub fn z80() -> Self {
        Self {
            kind: Intel8080Kind::Zilog80,
        }
    }

    pub fn i8080() -> Self {
        Self {
            kind: Intel8080Kind::Intel8080,
        }
    }
}

impl<P: Platform> ComponentConfig<P> for Intel8080Config {
    type Component = Intel8080;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
        _save: Option<&ComponentSave>,
    ) -> Result<(), BuildError> {
        todo!()
    }
}
