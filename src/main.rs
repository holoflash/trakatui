mod app;
mod audio;
mod export;
mod keys;
mod pattern;
mod scale;
mod synth;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("TRAKATUI"),
        ..Default::default()
    };

    eframe::run_native(
        "trakatui",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(TrakatuiApp::new()))
        }),
    )
}

struct TrakatuiApp {
    app: app::App,
}

impl TrakatuiApp {
    fn new() -> Self {
        Self {
            app: app::App::new(),
        }
    }
}

impl eframe::App for TrakatuiApp {
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
