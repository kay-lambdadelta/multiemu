use std::{collections::HashMap, marker::PhantomData, sync::Weak};

use multiemu_definition_mos6502::Mos6502;
use multiemu_runtime::{
    component::{ComponentConfig, ComponentPath, LateInitializedData},
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    memory::AddressSpaceId,
    platform::Platform,
    scheduler::Period,
};
use nalgebra::Point2;
use strum::IntoEnumIterator;

use super::{Tia, region::Region};
use crate::tia::{
    InputControl,
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

    fn late_initialize(component: &mut Self::Component, data: &LateInitializedData<P>) {
        component.backend = Some(TiaDisplayBackend::new(
            data.component_graphics_initialization_data.clone(),
        ));

        component.machine = data.machine.clone();
    }

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let (mut component_builder, _) = component_builder
            .set_scheduler_participation(SchedulerParticipation::OnDemand)
            .insert_display("tv");

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
            .interact::<Mos6502, _>(&self.cpu, |cpu| cpu.rdy())
            .unwrap();

        Ok(Tia {
            backend: None,
            cpu_rdy,
            collision_matrix: HashMap::default(),
            vblank_active: false,
            cycles_waiting_for_vsync: None,
            input_control: [InputControl::default(); 6],
            electron_beam: Point2::default(),
            missiles: Default::default(),
            ball: Default::default(),
            players: Default::default(),
            playfield: Default::default(),
            high_playfield_ball_priority: false,
            background_color: Default::default(),
            machine: Weak::new(),
            my_path: component_builder.path().clone(),
            timestamp: Period::default(),
        })
    }
}
