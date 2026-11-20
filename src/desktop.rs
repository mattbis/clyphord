use aiclip::{publish, render, Entry, Store};
use anyhow::{ensure, Result};
use eframe::egui::{self, ViewportBuilder, ViewportCommand, ViewportId, WindowLevel};

struct Desk {
    store: Store,
    session: String,
    buffer: String,
    source: String,
    note: String,
    preview: String,
    status: String,
    compare: bool,
    compare_session: String,
    compare_buffer: String,
    compare_preview: String,
    detached: bool,
    pinned: bool,
}
impl Desk {
    fn new(store: Store, session: String) -> Self {
        Self {
            store,
            compare_session: session.clone(),
            session,
            buffer: "review".into(),
            source: String::new(),
            note: String::new(),
            preview: String::new(),
            status: String::new(),
            compare: false,
            compare_buffer: "response".into(),
            compare_preview: String::new(),
            detached: false,
            pinned: false,
        }
    }
    fn refresh(&mut self) -> Result<()> {
        self.preview = render(&self.store.read(&self.session, &self.buffer)?, true, "\n\n")?.text;
        Ok(())
    }
    fn action(&mut self, action: &str) -> Result<()> {
        ensure!(
            !self.session.trim().is_empty() && !self.buffer.trim().is_empty(),
            "Enter a session and buffer name"
        );
        match action {
            "Add source" => {
                ensure!(!self.source.is_empty(), "Enter a path or wildcard");
                let e = Entry::source(&self.source)?;
                render(std::slice::from_ref(&e), true, "\n\n")?;
                self.store.add(&self.session, &self.buffer, &[e])?;
                self.source.clear();
            }
            "Add text" => {
                ensure!(!self.note.is_empty(), "Enter instructions");
                self.store.add(
                    &self.session,
                    &self.buffer,
                    &[Entry::new("literal", self.note.clone())],
                )?;
                self.note.clear();
            }
            "Capture" => {
                let text = arboard::Clipboard::new()?.get_text()?;
                self.store
                    .add(&self.session, &self.buffer, &[Entry::new("capture", text)])?;
            }
            "Publish" => {
                self.refresh()?;
                return publish(&self.preview);
            }
            "Refresh" => {}
            _ => anyhow::bail!("unknown action {action}"),
        }
        self.refresh()
    }
    fn button(&mut self, ui: &mut egui::Ui, label: &str) {
        if ui.button(label).clicked() {
            self.status = match self.action(label) {
                Ok(()) => format!("{label} complete"),
                Err(e) => format!("{e:#}"),
            };
        }
    }
    fn buffer_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for label in ["Refresh", "Publish", "Capture"] {
                self.button(ui, label);
            }
        });
        ui.label(&self.status);
        ui.checkbox(&mut self.compare, "Compare buffers");
        if self.compare {
            ui.horizontal_wrapped(|ui| {
                ui.label("Compare with session");
                let session = ui
                    .add(egui::TextEdit::singleline(&mut self.compare_session).desired_width(90.0));
                ui.label("Buffer");
                let buffer = ui
                    .add(egui::TextEdit::singleline(&mut self.compare_buffer).desired_width(90.0));
                if session.changed() || buffer.changed() {
                    self.compare_preview.clear();
                }
                if ui.button("Load both").clicked() {
                    let load = || -> Result<(String, String)> {
                        let left =
                            render(&self.store.read(&self.session, &self.buffer)?, true, "\n\n")?;
                        let right = render(
                            &self
                                .store
                                .read(&self.compare_session, &self.compare_buffer)?,
                            true,
                            "\n\n",
                        )?;
                        Ok((left.text, right.text))
                    };
                    match load() {
                        Ok((left, right)) => {
                            self.preview = left;
                            self.compare_preview = right;
                            self.status.clear();
                        }
                        Err(e) => {
                            self.compare_preview.clear();
                            self.status = format!("{e:#}");
                        }
                    }
                }
            });
            ui.columns(2, |columns| {
                for (index, (title, text)) in [
                    (&self.buffer, &self.preview),
                    (&self.compare_buffer, &self.compare_preview),
                ]
                .into_iter()
                .enumerate()
                {
                    columns[index].heading(title);
                    egui::ScrollArea::vertical()
                        .id_salt(index)
                        .show(&mut columns[index], |ui| {
                            ui.monospace(text);
                        });
                }
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.monospace(&self.preview);
            });
        }
    }
}
impl eframe::App for Desk {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Session");
                let session =
                    ui.add(egui::TextEdit::singleline(&mut self.session).desired_width(120.0));
                ui.label("Buffer");
                let buffer =
                    ui.add(egui::TextEdit::singleline(&mut self.buffer).desired_width(140.0));
                if session.changed() || buffer.changed() {
                    self.preview.clear();
                    self.status.clear();
                }
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.source).hint_text("File path or wildcard"),
                );
                self.button(ui, "Add source");
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.note)
                        .desired_rows(2)
                        .hint_text("Review instructions"),
                );
                self.button(ui, "Add text");
            });
            ui.separator();
            if self.detached {
                if ui.button("Reattach preview").clicked() {
                    self.detached = false;
                }
                ui.label(&self.status);
            } else {
                if ui.button("Detach preview").clicked() {
                    self.detached = true;
                }
                self.buffer_view(ui);
            }
        });
        if self.detached {
            let level = if self.pinned {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("session-preview"),
                ViewportBuilder::default()
                    .with_title("aiclip preview")
                    .with_decorations(false)
                    .with_window_level(level)
                    .with_inner_size([460.0, 320.0]),
                |child, _| {
                    egui::CentralPanel::default().show(child, |ui| {
                        ui.horizontal(|ui| {
                            let drag = ui.add(
                                egui::Label::new(format!("{} / {}", self.session, self.buffer))
                                    .sense(egui::Sense::drag()),
                            );
                            if drag.is_pointer_button_down_on()
                                && ui.input(|i| i.pointer.primary_pressed())
                            {
                                child.send_viewport_cmd(ViewportCommand::StartDrag);
                            }
                            if ui.toggle_value(&mut self.pinned, "Pin").changed() {
                                let level = if self.pinned {
                                    WindowLevel::AlwaysOnTop
                                } else {
                                    WindowLevel::Normal
                                };
                                child.send_viewport_cmd(ViewportCommand::WindowLevel(level));
                            }
                            if ui.button("Reattach").clicked() {
                                self.detached = false;
                            }
                        });
                        self.buffer_view(ui);
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
pub fn run(store: Store, session: String) -> eframe::Result<()> {
    eframe::run_native(
        "aiclip",
        eframe::NativeOptions::default(),
        Box::new(move |_| {
            let mut app = Desk::new(store, session);
            if app.store.buffers(&app.session)?.contains(&app.buffer) {
                app.refresh().unwrap_or_else(|e| app.status = e.to_string());
            }
            Ok(Box::new(app))
        }),
    )
}
