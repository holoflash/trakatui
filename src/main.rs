mod app;
mod audio;
mod export;
mod keys;
mod pattern;
mod scale;
mod synth;
mod ui;

use eframe::egui::{self, Color32, Stroke};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("psikat")
            .with_icon(make_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "psikat",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            cc.egui_ctx.set_visuals(egui::Visuals {
                selection: egui::style::Selection {
                    bg_fill: Color32::TRANSPARENT,
                    stroke: Stroke::NONE,
                },

                ..Default::default()
            });
            Ok(Box::new(PsikatApp::new()))
        }),
    )
}

struct PsikatApp {
    app: app::App,
}

impl PsikatApp {
    fn new() -> Self {
        Self {
            app: app::App::new(),
        }
    }
}

impl eframe::App for PsikatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let should_close = self.app.handle_input(ctx);
        if should_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        self.app.tick();

        ui::draw(ctx, &mut self.app);

        if self.app.playing {
            ctx.request_repaint();
        }
    }
}

fn make_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../psikat.png")).expect("Failed to load icon")
}
