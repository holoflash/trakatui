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
            .with_title("PSIKAT")
            .with_icon(make_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "psikat",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
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

// Temporary placeholder greek PSI letter ish
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

    for y in 4..28 {
        for x in 14..18 {
            set(&mut rgba, x, y, &fg);
        }
    }

    let left_arm: &[(u32, u32, u32, u32)] = &[
        (5, 8, 18, 20),
        (5, 8, 16, 18),
        (4, 7, 14, 16),
        (4, 7, 12, 14),
        (4, 7, 10, 12),
        (4, 7, 8, 10),
        (5, 8, 6, 8),
        (6, 9, 4, 6),
    ];
    for &(x0, x1, y0, y1) in left_arm {
        for y in y0..y1 {
            for x in x0..x1 {
                set(&mut rgba, x, y, &fg);
            }
        }
    }

    let right_arm: &[(u32, u32, u32, u32)] = &[
        (24, 27, 18, 20),
        (24, 27, 16, 18),
        (25, 28, 14, 16),
        (25, 28, 12, 14),
        (25, 28, 10, 12),
        (25, 28, 8, 10),
        (24, 27, 6, 8),
        (23, 26, 4, 6),
    ];
    for &(x0, x1, y0, y1) in right_arm {
        for y in y0..y1 {
            for x in x0..x1 {
                set(&mut rgba, x, y, &fg);
            }
        }
    }

    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
