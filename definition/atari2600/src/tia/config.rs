use std::marker::PhantomData;

use multiemu_definition_mos6502::Mos6502;
use multiemu_runtime::{
    component::{ComponentConfig, ComponentPath},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
};
use strum::IntoEnumIterator;

use super::{Tia, region::Region, task::TiaTask};
use crate::tia::{
    backend::{SupportedGraphicsApiTia, TiaDisplayBackend},
    memory::{ReadRegisters, WriteRegisters},
};

#[derive(Debug, Clone)]
pub(crate) struct TiaConfig<R: Region> {
    pub cpu: ComponentPath,
    pub cpu_address_space: AddressSpaceId,
    pub _phantom: PhantomData<R>,
}

impl<R: Region, P: Platform<GraphicsApi: SupportedGraphicsApiTia>> ComponentConfig<P>
    for TiaConfig<R>
{
    type Component = Tia<R, P::GraphicsApi>;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let (mut component_builder, _) = component_builder.insert_display("tv");

        for register in ReadRegisters::iter() {
            component_builder = component_builder.memory_map_component_read(
                self.cpu_address_space,
                register as usize..=register as usize,
            );
        }

        for register in WriteRegisters::iter() {
            component_builder = component_builder.memory_map_component_write(
                self.cpu_address_space,
                register as usize..=register as usize,
            );
        }

        let cpu_rdy = component_builder
            .registry()
            .interact::<Mos6502, _>(&self.cpu, |cpu| cpu.rdy())
            .unwrap();

        let (component_builder, _) =
            component_builder.insert_task("driver", R::frequency(), TiaTask);

        component_builder.set_lazy_component_initializer(move |component, lazy| {
            component.backend = Some(TiaDisplayBackend::new(
                lazy.component_graphics_initialization_data.clone(),
            ));
        });

        Ok(Tia {
            state: Default::default(),
            backend: None,
            cpu_rdy,
        })
    }
}
