mod snap;
use egui::{self, Id, ViewportBuilder, ViewportCommand, ViewportId, WindowLevel};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};

pub trait Panel {
    type Shared;
    fn title(&self) -> String;
    fn can_close(&self) -> bool {
        true
    }
    fn detached_preview(&self, _ui: &mut egui::Ui) {}
    fn strip_actions(&mut self, _ui: &mut egui::Ui, _shared: &mut Self::Shared) {}
    fn contents(&mut self, ui: &mut egui::Ui, shared: &mut Self::Shared);
}

struct Tab<T> {
    id: u64,
    panel: T,
    detached: bool,
    pinned: bool,
    compact: bool,
    expanded_size: egui::Vec2,
}

pub struct Workspace<T> {
    dock: DockState<Tab<T>>,
    next_id: u64,
    pub snap_pixels: u16,
}

impl<T: Panel> Workspace<T> {
    pub fn new(first: T) -> Self {
        Self {
            dock: DockState::new(vec![Tab {
                id: 0,
                panel: first,
                detached: false,
                pinned: false,
                compact: false,
                expanded_size: egui::vec2(560.0, 380.0),
            }]),
            next_id: 1,
            snap_pixels: 14,
        }
    }
    pub fn add(&mut self, panel: T, split: bool) {
        let tab = Tab {
            id: self.next_id,
            panel,
            detached: false,
            pinned: false,
            compact: false,
            expanded_size: egui::vec2(560.0, 380.0),
        };
        self.next_id += 1;
        let tree = self.dock.main_surface_mut();
        if split && !tree.is_empty() {
            let node = tree
                .focused_leaf()
                .or_else(|| tree.iter().position(|n| n.is_leaf()).map(NodeIndex))
                .expect("nonempty tree contains a leaf");
            tree.split_right(node, 0.5, vec![tab]);
        } else {
            self.dock.push_to_focused_leaf(tab);
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, shared: &mut T::Shared) {
        egui::CentralPanel::default().show(ctx, |ui| {
            DockArea::new(&mut self.dock)
                .style(Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut Viewer::<T>(shared));
        });
        for (_, tab) in self.dock.iter_all_tabs_mut() {
            if !tab.detached {
                continue;
            }
            let level = if tab.pinned {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            };
            let size = if tab.compact {
                egui::vec2(tab.expanded_size.x.max(520.0), 52.0)
            } else {
                tab.expanded_size
            };
            let min_size = if tab.compact {
                egui::vec2(520.0, 52.0)
            } else {
                egui::vec2(320.0, 180.0)
            };
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of(("workspace-panel", tab.id)),
                ViewportBuilder::default()
                    .with_title(tab.panel.title())
                    .with_decorations(false)
                    .with_inner_size(size)
                    .with_min_inner_size(min_size)
                    .with_resizable(!tab.compact)
                    .with_window_level(level),
                |child, _| {
                    if !tab.compact {
                        if let Some(size) =
                            child.input(|i| i.viewport().inner_rect.map(|r| r.size()))
                        {
                            if size.y >= 180.0 {
                                tab.expanded_size = size;
                            }
                        }
                    }
                    if child.input(|i| !i.events.is_empty()) {
                        child.request_repaint_of(ViewportId::ROOT);
                    }
                    egui::TopBottomPanel::top("panel-controls").show(child, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Reattach").clicked() {
                                tab.detached = false;
                            }
                            if ui.toggle_value(&mut tab.pinned, "Pin").changed() {
                                child.send_viewport_cmd(ViewportCommand::WindowLevel(
                                    if tab.pinned {
                                        WindowLevel::AlwaysOnTop
                                    } else {
                                        WindowLevel::Normal
                                    },
                                ));
                            }
                            if ui
                                .button(if tab.compact { "Expand" } else { "Strip" })
                                .clicked()
                            {
                                tab.compact = !tab.compact;
                                child.request_repaint_of(ViewportId::ROOT);
                            }
                            if tab.compact {
                                tab.panel.strip_actions(ui, shared);
                            }
                            let grip = ui
                                .add_sized(
                                    [ui.available_width(), 26.0],
                                    egui::Label::new(
                                        egui::RichText::new(tab.panel.title()).strong(),
                                    )
                                    .sense(egui::Sense::drag()),
                                )
                                .on_hover_cursor(egui::CursorIcon::Grab);
                            if grip.is_pointer_button_down_on()
                                && ui.input(|i| i.pointer.primary_pressed())
                            {
                                if self.snap_pixels > 0 {
                                    snap::arm(self.snap_pixels);
                                }
                                child.send_viewport_cmd(ViewportCommand::StartDrag);
                            }
                        });
                    });
                    if !tab.compact {
                        egui::CentralPanel::default()
                            .show(child, |ui| tab.panel.contents(ui, shared));
                    }
                    if child.input(|i| i.viewport().close_requested()) {
                        tab.detached = false;
                    }
                    if !tab.detached {
                        child.request_repaint_of(ViewportId::ROOT);
                    }
                },
            );
        }
    }
}
struct Viewer<'a, T: Panel>(&'a mut T::Shared);
impl<T: Panel> TabViewer for Viewer<'_, T> {
    type Tab = Tab<T>;
    fn title(&mut self, t: &mut Self::Tab) -> egui::WidgetText {
        t.panel.title().into()
    }
    fn id(&mut self, t: &mut Self::Tab) -> Id {
        Id::new(("panel", t.id))
    }
    fn allowed_in_windows(&self, _: &mut Self::Tab) -> bool {
        false
    }
    fn closeable(&mut self, t: &mut Self::Tab) -> bool {
        !t.detached && t.panel.can_close()
    }
    fn scroll_bars(&self, _: &Self::Tab) -> [bool; 2] {
        [false, false]
    }
    fn ui(&mut self, ui: &mut egui::Ui, t: &mut Self::Tab) {
        if t.detached {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Reattach panel").clicked() {
                    t.detached = false;
                }
                ui.toggle_value(&mut t.pinned, "Pin window");
                ui.toggle_value(&mut t.compact, "Compact strip");
            });
            t.panel.detached_preview(ui);
        } else {
            if ui.button("Detach panel").clicked() {
                t.detached = true;
            }
            t.panel.contents(ui, self.0);
        }
    }
}
