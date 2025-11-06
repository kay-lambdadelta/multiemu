use egui_snarl::{
    Snarl,
    ui::{PinInfo, SnarlPin, SnarlStyle, SnarlViewer},
};
use multiemu_runtime::input::{Input, RealGamepadMetadata};

#[derive(Debug)]
pub struct GamepadConfigState {
    style: SnarlStyle,
}

impl GamepadConfigState {
    pub fn new() -> Self {
        Self {
            style: SnarlStyle {
                ..Default::default()
            },
        }
    }

    pub fn run(&mut self, ui: &mut egui::Ui) {}
}

#[derive(Debug)]
struct GamepadNode<'a> {
    name: &'a str,
    ty: NodeType<'a>,
}

#[derive(Debug)]
enum NodeType<'a> {
    Real(&'a RealGamepadMetadata),
    Virtual { present_inputs: Vec<Input> },
}

#[derive(Debug)]
struct Viewer;

impl SnarlViewer<GamepadNode<'_>> for Viewer {
    fn title(&mut self, node: &GamepadNode) -> String {
        node.name.to_string()
    }

    fn inputs(&mut self, node: &GamepadNode) -> usize {
        match &node.ty {
            NodeType::Real(_) => 0,
            NodeType::Virtual { present_inputs } => present_inputs.len(),
        }
    }

    fn show_input(
        &mut self,
        pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<GamepadNode>,
    ) -> impl SnarlPin + 'static {
        let id = pin.id.node;
        ui.label(snarl.get_node(id).unwrap().name);

        PinInfo::default()
    }

    fn outputs(&mut self, node: &GamepadNode) -> usize {
        match &node.ty {
            NodeType::Real(metadata) => metadata.present_inputs.len(),
            NodeType::Virtual { .. } => 0,
        }
    }

    fn show_output(
        &mut self,
        pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<GamepadNode>,
    ) -> impl SnarlPin + 'static {
        PinInfo::default()
    }
}
