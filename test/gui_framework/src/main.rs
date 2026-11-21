use aiclip_window_workspace::{Panel, Workspace};
use eframe::egui;

struct Editor {
    name: String,
    text: String,
}
impl Panel for Editor {
    type Shared = ();
    fn title(&self) -> String {
        self.name.clone()
    }
    fn detached_preview(&self, ui: &mut egui::Ui) {
        ui.label(&self.text);
    }
    fn contents(&mut self, ui: &mut egui::Ui, _: &mut ()) {
        ui.add_sized(
            ui.available_size(),
            egui::TextEdit::multiline(&mut self.text)
                .hint_text("Type here, detach the panel, then reattach it."),
        );
    }
}
struct Probe {
    workspace: Workspace<Editor>,
    count: usize,
}
impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("probe-tools").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (label, split) in [("New tab", false), ("Split right", true)] {
                    if ui.button(label).clicked() {
                        self.count += 1;
                        self.workspace.add(
                            Editor {
                                name: format!("Buffer {}", self.count),
                                text: String::new(),
                            },
                            split,
                        );
                    }
                }
            });
        });
        self.workspace.show(ctx, &mut ());
    }
}
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "aiclip GUI smoke test",
        Default::default(),
        Box::new(|_| {
            Ok(Box::new(Probe {
                workspace: Workspace::new(Editor {
                    name: "Buffer 1".into(),
                    text: String::new(),
                }),
                count: 1,
            }))
        }),
    )
}
