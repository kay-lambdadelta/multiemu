use super::{SupportedRenderApiTia, Tia, memory::MemoryCallback, region::Region, task::TiaTask};
use crate::tia::TiaDisplayBackend;
use multiemu_definition_mos6502::Mos6502;
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{ComponentConfig, component_ref::ComponentRef},
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

impl<R: Region, A: SupportedRenderApiTia, B: ComponentBuilder<Component = Tia<R, A>, RenderApi = A>>
    ComponentConfig<B> for TiaConfig<R>
{
    type Component = Tia<R, A>;

    fn build_component(self, component_builder: B) -> B::BuildOutput {
        let state: Arc<Mutex<_>> = Arc::default();
        let essentials = component_builder.essentials();

        let component_builder =
            component_builder.insert_display_config(None, None, move |component: &Tia<R, A>| {
                let (backend, framebuffer) =
                    <A::Backend<R> as TiaDisplayBackend<R, A>>::new(essentials.as_ref());

                component.display_backend.set(backend).unwrap();

                framebuffer
            });

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
                cpu: self.cpu.clone(),
            },
        );

        component_builder.build(Tia {
            state,
            display_backend: OnceCell::default(),
        })
    }
}
