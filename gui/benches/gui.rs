use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use egui::{Context, RawInput, Rect, SidePanel, ViewportId};
use multiemu_gui::SoftwareEguiRenderer;
use nalgebra::{DMatrix, Vector2};
use palette::{cast::Packed, rgb::channels::Argb, Srgba};

const RESOLUTIONS: &[Vector2<usize>] = &[
    Vector2::new(640, 480),
    Vector2::new(800, 600),
    Vector2::new(1280, 720),
    Vector2::new(1280, 800),
    Vector2::new(1920, 1080),
];

fn criterion_benchmark(c: &mut Criterion) {
    for resolution in RESOLUTIONS {
        let mut software_egui_renderer = SoftwareEguiRenderer::default();
        let egui_context = Context::default();

        let mut render_buffer = DMatrix::from_element(
            resolution.x,
            resolution.y,
            Packed::pack(Srgba::new(0, 0, 0, 255)),
        );
        let output = egui_context.run(
            RawInput {
                viewport_id: ViewportId::ROOT,
                screen_rect: Some(Rect::from_min_max(
                    (0.0, 0.0).into(),
                    (resolution.x as f32, resolution.y as f32).into(),
                )),
                ..Default::default()
            },
            gui_main,
        );

        c.bench_function(&format!("{}x{}", resolution.x, resolution.y), |b| {
            b.iter(|| {
                software_egui_renderer.render::<Argb>(
                    &egui_context,
                    black_box(render_buffer.as_view_mut()),
                    output.clone(),
                );
            })
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn gui_main(egui_context: &egui::Context) {
    SidePanel::left("side_panel").show(egui_context, |ui| {
        ui.label("Side panel");
    });

    SidePanel::right("right_panel").show(egui_context, |ui| {
        ui.label("Right panel");
    });

    egui::CentralPanel::default().show(egui_context, |ui| {
        ui.label("Central panel");
    });

    egui::TopBottomPanel::top("top_panel").show(egui_context, |ui| {
        ui.label("Top panel");
    });

    egui::TopBottomPanel::bottom("bottom_panel").show(egui_context, |ui| {
        ui.label("Bottom panel");
    });
}
