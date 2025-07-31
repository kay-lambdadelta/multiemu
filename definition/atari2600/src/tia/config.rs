use super::{Tia, region::Region, task::TiaTask};
use crate::tia::backend::{SupportedGraphicsApiTia, TiaDisplayBackend};
use multiemu_definition_mos6502::Mos6502;
use multiemu_graphics::GraphicsApi;
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, ComponentConfig, ComponentRef},
    graphics::DisplayCallback,
    memory::AddressSpaceHandle,
    platform::Platform,
};
use std::{marker::PhantomData, sync::Mutex};

#[derive(Debug, Clone)]
pub(crate) struct TiaConfig<R: Region> {
    pub cpu: ComponentRef<Mos6502>,
    pub cpu_address_space: AddressSpaceHandle,
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

        let (component_builder, _) = component_builder.insert_display(TiaDisplayCallback {
            component: component.clone(),
        });

        let component_builder =
            component_builder.map_memory([(self.cpu_address_space, 0x000..=0x03f)]);

        let component_builder = component_builder.insert_task(
            R::frequency(),
            "tia",
            TiaTask {
                component: component,
                cpu: self.cpu.clone(),
            },
        );

        component_builder.build(|lazy| Tia {
            state: Default::default(),
            backend: Mutex::new(TiaDisplayBackend::new(
                lazy.component_graphics_initialization_data.clone(),
            )),
            config: self,
        });

        Ok(())
    }
}

#[derive(Debug)]
struct TiaDisplayCallback<R: Region, G: SupportedGraphicsApiTia> {
    component: ComponentRef<Tia<R, G>>,
}

impl<R: Region, G: SupportedGraphicsApiTia> DisplayCallback<G> for TiaDisplayCallback<R, G> {
    fn access_framebuffer(
        &self,
        callback: Box<dyn FnOnce(&<G as GraphicsApi>::FramebufferTexture) + '_>,
    ) {
        self.component
            .interact_local(|component| {
                component
                    .backend
                    .lock()
                    .unwrap()
                    .access_framebuffer(callback);
            })
            .unwrap()
    }
}
