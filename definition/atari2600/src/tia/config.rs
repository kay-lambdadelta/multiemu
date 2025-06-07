use crate::tia::TiaDisplayBackend;

use super::{SupportedRenderApiTia, Tia, memory::MemoryCallback, region::Region, task::TiaTask};
use multiemu_definition_mos6502::Mos6502;
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{ComponentConfig, component_ref::ComponentRef},
    graphics::GraphicsCallback,
    memory::memory_translation_table::address_space::AddressSpaceHandle,
};
use std::{
    cell::OnceCell,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub(crate) struct TiaConfig<R: Region> {
    pub cpu: ComponentRef<Mos6502>,
    pub cpu_address_space: AddressSpaceHandle,
    pub _phantom: PhantomData<R>,
}

impl<
    R: Region,
    A: SupportedRenderApiTia,
    B: ComponentBuilder<Component = Tia<R, A>, GraphicsApi = A>,
> ComponentConfig<B> for TiaConfig<R>
{
    type Component = Tia<R, A>;

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: B,
    ) -> B::BuildOutput {
        let state: Arc<Mutex<_>> = Arc::default();
        let essentials = component_builder.essentials();

        let component_builder = component_builder.insert_screen(
            None,
            None,
            TiaGraphicsCallback {
                component: component_ref.clone(),
            },
        );

        let (component_builder, _) = component_builder.insert_memory(
            MemoryCallback {
                state: state.clone(),
                cpu: self.cpu.clone(),
            },
            [(self.cpu_address_space, 0x000..=0x03f)],
        );

        let component_builder = component_builder.insert_task(
            R::frequency(),
            TiaTask {
                component: component_ref.clone(),
                cpu: self.cpu.clone(),
            },
        );

        component_builder.build(Tia {
            state,
            backend: OnceCell::default(),
            essentials,
        })
    }
}

struct TiaGraphicsCallback<R: Region, A: SupportedRenderApiTia> {
    component: ComponentRef<Tia<R, A>>,
}

impl<R: Region, A: SupportedRenderApiTia> GraphicsCallback<A> for TiaGraphicsCallback<R, A> {
    fn get_framebuffer<'a>(&'a self, callback: Box<dyn FnOnce(&A::ComponentFramebuffer) + 'a>) {
        self.component
            .interact_local(|component| {
                component.backend.get().unwrap().get_framebuffer(callback);
            })
            .unwrap();
    }
}
