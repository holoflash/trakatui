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
            .with_title("TRAKATUI")
            .with_icon(make_icon()),
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

fn make_icon() -> egui::IconData {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    let bg = [18u8, 18, 24, 255];
    let fg = [80u8, 200, 220, 255];

    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&bg);
    }

    let set = |rgba: &mut [u8], x: u32, y: u32, color: &[u8; 4]| {
        let i = ((y * size + x) * 4) as usize;
        rgba[i..i + 4].copy_from_slice(color);
    };

    // top bar of T: row 6-9, cols 6-25
    for y in 6..10 {
        for x in 6..26 {
            set(&mut rgba, x, y, &fg);
        }
    }

    // vertical stem of T: rows 10-25, cols 14-17
    for y in 10..26 {
        for x in 14..18 {
            set(&mut rgba, x, y, &fg);
        }
    }

    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
