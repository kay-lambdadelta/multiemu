use super::{Tia, region::Region, task::TiaTask};
use crate::tia::backend::{SupportedGraphicsApiTia, TiaDisplayBackend};
use multiemu_definition_mos6502::Mos6502;
use multiemu_runtime::{
    component::{ComponentConfig, ComponentPath},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
};
use std::marker::PhantomData;

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
        let (component_builder, _) = component_builder.insert_display("tv");
        let component_builder = component_builder.memory_map(0x000..=0x03f, self.cpu_address_space);

        let cpu_rdy = component_builder
            .registry()
            .interact::<Mos6502, _>(&self.cpu, |cpu| cpu.rdy())
            .unwrap();

        let component_builder = component_builder.insert_task_mut(
            "driver",
            R::frequency(),
            TiaTask {
                cpu_rdy: cpu_rdy.clone(),
            },
        );

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
