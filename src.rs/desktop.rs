use anyhow::{ensure, Result};
use clyphord::{publish, render, Entry, Store};
use clyphord_window_workspace::{Panel, Workspace};
use eframe::egui;

struct BufferPanel {
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
}
impl BufferPanel {
    fn new(session: String, buffer: String) -> Self {
        Self {
            compare_session: session.clone(),
            session,
            buffer,
            source: String::new(),
            note: String::new(),
            preview: String::new(),
            status: String::new(),
            compare: false,
            compare_buffer: "response".into(),
            compare_preview: String::new(),
        }
    }
    fn refresh(&mut self, store: &Store) -> Result<()> {
        self.preview = render(&store.read(&self.session, &self.buffer)?, true, "\n\n")?.text;
        Ok(())
    }
    fn action(&mut self, action: &str, store: &mut Store) -> Result<()> {
        ensure!(
            !self.session.trim().is_empty() && !self.buffer.trim().is_empty(),
            "Enter a session and buffer name"
        );
        match action {
            "Add source" => {
                ensure!(!self.source.is_empty(), "Enter a path or wildcard");
                let e = Entry::source(&self.source)?;
                render(std::slice::from_ref(&e), true, "\n\n")?;
                store.add(&self.session, &self.buffer, &[e])?;
                self.source.clear();
            }
            "Add text" => {
                ensure!(!self.note.is_empty(), "Enter instructions");
                store.add(
                    &self.session,
                    &self.buffer,
                    &[Entry::new("literal", self.note.clone())],
                )?;
                self.note.clear();
            }
            "Capture" => {
                let text = arboard::Clipboard::new()?.get_text()?;
                store.add(&self.session, &self.buffer, &[Entry::new("capture", text)])?;
            }
            "Publish" => {
                self.refresh(store)?;
                return publish(&self.preview);
            }
            "Refresh" => {}
            _ => anyhow::bail!("unknown action {action}"),
        }
        self.refresh(store)
    }
    fn button(&mut self, ui: &mut egui::Ui, label: &str, store: &mut Store) {
        if ui.button(label).clicked() {
            self.status = match self.action(label, store) {
                Ok(()) => format!("{label} complete"),
                Err(e) => format!("{e:#}"),
            };
        }
    }
    fn buffer_view(&mut self, ui: &mut egui::Ui, store: &mut Store) {
        ui.horizontal_wrapped(|ui| {
            for label in ["Refresh", "Publish", "Capture"] {
                self.button(ui, label, store);
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
                        let left = render(&store.read(&self.session, &self.buffer)?, true, "\n\n")?;
                        let right = render(
                            &store.read(&self.compare_session, &self.compare_buffer)?,
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

impl Panel for BufferPanel {
    type Shared = Store;
    fn title(&self) -> String {
        format!("{} / {}", self.session, self.buffer)
    }
    fn strip_actions(&mut self, ui: &mut egui::Ui, store: &mut Store) {
        self.button(ui, "Publish", store);
        self.button(ui, "Capture", store);
        if !self.status.is_empty() {
            ui.label("ⓘ").on_hover_text(&self.status);
        }
    }
    fn detached_preview(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.monospace(&self.preview);
        });
    }
    fn can_close(&self) -> bool {
        self.note.is_empty() && self.source.is_empty()
    }
    fn contents(&mut self, ui: &mut egui::Ui, store: &mut Store) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Session");
            let session =
                ui.add(egui::TextEdit::singleline(&mut self.session).desired_width(120.0));
            ui.label("Buffer");
            let buffer = ui.add(egui::TextEdit::singleline(&mut self.buffer).desired_width(140.0));
            if session.changed() || buffer.changed() {
                self.preview.clear();
                self.status.clear();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.source).hint_text("File path or wildcard"));
            self.button(ui, "Add source", store);
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.note)
                    .desired_rows(2)
                    .hint_text("Review instructions"),
            );
            self.button(ui, "Add text", store);
        });
        ui.separator();
        self.buffer_view(ui, store);
    }
}

struct Desk {
    store: Store,
    workspace: Workspace<BufferPanel>,
    session: String,
}
impl Desk {
    fn panel(store: &Store, session: &str, name: &str) -> BufferPanel {
        let mut panel = BufferPanel::new(session.into(), name.into());
        match store.buffers(session) {
            Ok(names) if names.contains(&panel.buffer) => {
                if let Err(err) = panel.refresh(store) {
                    panel.status = format!("{err:#}");
                }
            }
            Err(err) => panel.status = format!("{err:#}"),
            _ => {}
        }
        panel
    }
}
impl eframe::App for Desk {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("workspace-tools").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("clyphord");
                ui.separator();
                if ui.button("New tab").clicked() {
                    self.workspace
                        .add(Self::panel(&self.store, &self.session, "review"), false);
                }
                if ui.button("Split right").clicked() {
                    self.workspace
                        .add(Self::panel(&self.store, &self.session, "response"), true);
                }
                if ui.button("Fullscreen").clicked() {
                    let current = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!current));
                }
                #[cfg(windows)]
                ui.add(
                    egui::DragValue::new(&mut self.workspace.snap_pixels)
                        .range(0..=100)
                        .prefix("Snap ")
                        .suffix(" px"),
                )
                .on_hover_text("Pop-out edge distance. Set to zero to disable.");
            });
        });
        self.workspace.show(ctx, &mut self.store);
    }
}
pub fn run(store: Store, session: String) -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([960.0, 620.0])
        .with_min_inner_size([420.0, 320.0]);
    eframe::run_native(
        "clyphord",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| {
            cc.egui_ctx.style_mut(|s| {
                s.spacing.item_spacing = egui::vec2(8.0, 6.0);
                s.visuals.selection.bg_fill = egui::Color32::from_rgb(42, 95, 101);
            });
            let workspace = Workspace::new(Desk::panel(&store, &session, "review"));
            Ok(Box::new(Desk {
                store,
                workspace,
                session,
            }))
        }),
    )
}
