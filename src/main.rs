mod app;
mod audio;

mod project;
mod ui;

use eframe::egui::{self, Color32, Stroke};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_min_inner_size([750.0, 400.0])
            .with_title("psikat")
            .with_icon(make_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "psikat",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let mut defaults = egui::Visuals::default();
            defaults.widgets.noninteractive.bg_stroke =
                Stroke::new(1.0, ui::COLOR_PATTERN_BEATMARKER);
            cc.egui_ctx.set_visuals(egui::Visuals {
                selection: egui::style::Selection {
                    bg_fill: Color32::TRANSPARENT,
                    stroke: Stroke::NONE,
                },
                widgets: defaults.widgets,
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
        if ctx.input(|i| i.viewport().close_requested()) && self.app.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.app.show_quit_confirm = true;
        }

        let should_close = self.app.handle_input(ctx);
        if should_close {
            if self.app.dirty {
                self.app.show_quit_confirm = true;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        self.app.tick();

        ui::draw(ctx, &mut self.app);

        if self.app.show_quit_confirm {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Do you want to save the changes you made to \"{}\"?",
                        self.app.project_name()
                    ));
                    ui.label("Your changes will be lost if you don't save them.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.app.show_quit_confirm = false;
                            self.app.do_quick_save();
                            if !self.app.dirty {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                        if ui.button("Don't Save").clicked() {
                            self.app.show_quit_confirm = false;
                            self.app.dirty = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancel").clicked() {
                            self.app.show_quit_confirm = false;
                        }
                    });
                });
        }

        if self.app.show_new_confirm {
            egui::Window::new("New Project")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Do you want to save the changes you made to \"{}\"?",
                        self.app.project_name()
                    ));
                    ui.label("Your changes will be lost if you don't save them.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.app.show_new_confirm = false;
                            self.app.do_quick_save();
                            if !self.app.dirty {
                                self.app.reset_project();
                            }
                        }
                        if ui.button("Don't Save").clicked() {
                            self.app.show_new_confirm = false;
                            self.app.reset_project();
                        }
                        if ui.button("Cancel").clicked() {
                            self.app.show_new_confirm = false;
                        }
                    });
                });
        }

        if self.app.playback.playing
            || self
                .app
                .display_scopes
                .iter()
                .any(|s| s.iter().any(|v| *v != 0.0))
        {
            ctx.request_repaint();
        }
    }
}

fn make_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/psikat_icon_256.png"))
        .expect("Failed to load icon")
}
