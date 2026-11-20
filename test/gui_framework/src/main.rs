use eframe::egui::{self, ViewportBuilder, ViewportCommand, ViewportId, WindowLevel};

#[derive(Default)]
struct SmokeTest {
    text: String,
    detached: bool,
    pinned: bool,
}

impl eframe::App for SmokeTest {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("aiclip window smoke test");
            if self.detached {
                ui.label("Live buffer preview");
                ui.label(&self.text);
                if ui.button("Reattach buffer").clicked() {
                    self.detached = false;
                }
            } else {
                ui.text_edit_multiline(&mut self.text);
                if ui.button("Detach buffer").clicked() {
                    self.detached = true;
                }
            }
        });

        if self.detached {
            let id = ViewportId::from_hash_of("buffer");
            let level = if self.pinned {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            ctx.show_viewport_immediate(
                id,
                ViewportBuilder::default()
                    .with_title("aiclip buffer")
                    .with_inner_size([400.0, 240.0])
                    .with_min_inner_size([260.0, 130.0])
                    .with_decorations(false)
                    .with_window_level(level),
                |child, _| {
                    egui::TopBottomPanel::top("buffer_header").show(child, |ui| {
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Reattach").clicked() {
                                        self.detached = false;
                                    }
                                    if ui.toggle_value(&mut self.pinned, "Pin").changed() {
                                        let level = if self.pinned {
                                            WindowLevel::AlwaysOnTop
                                        } else {
                                            WindowLevel::Normal
                                        };
                                        child
                                            .send_viewport_cmd(ViewportCommand::WindowLevel(level));
                                    }
                                    let grip = ui
                                        .add_sized(
                                            [ui.available_width(), 26.0],
                                            egui::Label::new(
                                                egui::RichText::new("Buffer").strong(),
                                            )
                                            .sense(egui::Sense::drag()),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::Grab);
                                    if grip.is_pointer_button_down_on()
                                        && ui.input(|i| i.pointer.primary_pressed())
                                    {
                                        child.send_viewport_cmd(ViewportCommand::StartDrag);
                                    }
                                },
                            );
                        });
                    });
                    egui::CentralPanel::default().show(child, |ui| {
                        if ui
                            .add_sized(
                                ui.available_size(),
                                egui::TextEdit::multiline(&mut self.text),
                            )
                            .changed()
                        {
                            child.request_repaint_of(ViewportId::ROOT);
                        }
                    });
                    if child.input(|i| i.viewport().close_requested()) {
                        self.detached = false;
                    }
                    if !self.detached {
                        child.request_repaint_of(ViewportId::ROOT);
                    }
                },
            );
        }
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "aiclip GUI smoke test",
        Default::default(),
        Box::new(|_| Ok(Box::<SmokeTest>::default())),
    )
}
