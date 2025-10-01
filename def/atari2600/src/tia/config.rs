use super::{Tia, region::Region, task::TiaTask};
use crate::tia::backend::{SupportedGraphicsApiTia, TiaDisplayBackend};
use multiemu::{
    component::{BuildError, ComponentConfig, ComponentRef},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
};
use multiemu_definition_mos6502::Mos6502;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub(crate) struct TiaConfig<R: Region> {
    pub cpu: ComponentRef<Mos6502>,
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
    ) -> Result<(), BuildError> {
        let component = component_builder.component_ref();

        let (component_builder, _) = component_builder.insert_display("tv");
        let component_builder = component_builder.memory_map(self.cpu_address_space, 0x000..=0x03f);

        let cpu_rdy = self.cpu.interact_local(|cpu| cpu.rdy()).unwrap();

        let component_builder = component_builder.insert_task(
            "driver",
            R::frequency(),
            TiaTask {
                component: component.clone(),
                cpu_rdy: cpu_rdy.clone(),
            },
        );

        component_builder
            .set_lazy_component_initializer(move |lazy| {
                component
                    .interact_local_mut(|tia| {
                        tia.backend = Some(TiaDisplayBackend::new(
                            lazy.component_graphics_initialization_data.clone(),
                        ));
                    })
                    .unwrap();
            })
            .build_local(Tia {
                state: Default::default(),
                backend: None,
                cpu_rdy,
            });

        Ok(())
    }
}
