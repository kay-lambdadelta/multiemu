use multiemu::{
    component::{BuildError, Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    platform::Platform,
};

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
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, BuildError> {
        todo!()
    }
}
