use std::process;

use eframe::egui;
use egui::RichText;

use super::{cross_centered, increase_font_size_by};
use crate::utils::pick_nsp_file;

#[derive(Debug, Default)]
pub struct YanuApp {
    page: Page,

    // Update/Unpack Page
    base_pkg_path: String,
    update_pkg_path: String,

    // Pack Page
    control_nca_path: String,
    pack_title_id: String,
    romfs_dir: String,
    exefs_dir: String,

    // Convert Page
    convert_kind: ConvertKind,
}

#[derive(Debug, Default, PartialEq)]
enum Page {
    #[default]
    Update,
    Unpack,
    Pack,
    Convert,
}

#[derive(Debug, Default, PartialEq)]
enum ConvertKind {
    #[default]
    Nsp,
}

impl YanuApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        increase_font_size_by(1.2, &cc.egui_ctx);

        Default::default()
    }
}

const HEADING_SIZE: f32 = 21.6; // 1.2x of default
const VERTICAL_PADDING: f32 = 6.25; // 0.5x of default Body size

impl eframe::App for YanuApp {
    /// Called by the frame work to save state before shutdown.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, frame.info().native_pixels_per_point);

        self.top_bar(ctx, frame);

        egui::SidePanel::left("actions_panel").show(ctx, |ui| {
            egui::ScrollArea::new([true, true])
                .auto_shrink([true, true])
                .show(ui, |ui| {
                    ui.selectable_value(&mut self.page, Page::Update, "Update");
                    ui.selectable_value(&mut self.page, Page::Unpack, "Unpack");
                    ui.selectable_value(&mut self.page, Page::Pack, "Pack");
                    ui.selectable_value(&mut self.page, Page::Convert, "Convert");
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Update => {
                cross_centered("center update", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Base Package:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.base_pkg_path);
                                if ui.button("📂 Browse").clicked() {
                                    match pick_nsp_file() {
                                        Ok(path) => {
                                            self.base_pkg_path = path.to_string_lossy().into();
                                        }
                                        Err(_) => todo!("error dialog popup"),
                                    }
                                };
                            });

                            ui.label("Update Package:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.update_pkg_path);
                                if ui.button("📂 Browse").clicked() {};
                            });
                        });
                    });

                    ui.add_space(VERTICAL_PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Update").size(HEADING_SIZE))
                            .clicked()
                        {};
                    });
                });
            }
            Page::Unpack => {
                cross_centered("center unpack", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Base Package:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.base_pkg_path);
                                if ui.button("📂 Browse").clicked() {};
                            });

                            ui.label("Update Package:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.update_pkg_path);
                                if ui.button("📂 Browse").clicked() {};
                            });
                        });
                    });

                    ui.add_space(VERTICAL_PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Unpack").size(HEADING_SIZE))
                            .clicked()
                        {};
                    });
                });
            }
            Page::Pack => {
                cross_centered("center pack", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Control NCA:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.control_nca_path);
                                if ui.button("📂 Browse").clicked() {};
                            });

                            ui.label("TitleID:");
                            ui.text_edit_singleline(&mut self.pack_title_id);

                            ui.label("RomFS directory:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.romfs_dir);
                                if ui.button("📂 Browse").clicked() {};
                            });

                            ui.label("ExeFS directory:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.exefs_dir);
                                if ui.button("📂 Browse").clicked() {};
                            });
                        });
                    });

                    ui.add_space(VERTICAL_PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Pack").size(HEADING_SIZE))
                            .clicked()
                        {};
                    });
                });
            }
            Page::Convert => {
                cross_centered("center convert", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Source:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.control_nca_path);
                                if ui.button("📂 Browse").clicked() {};
                            });

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.label("Convert to:");
                                egui::ComboBox::from_id_source("convert_kind")
                                    .selected_text(format!("{:?}", self.convert_kind))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.convert_kind,
                                            ConvertKind::Nsp,
                                            "Nsp",
                                        );
                                    });
                            });
                        });
                    });

                    ui.add_space(VERTICAL_PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Convert").size(HEADING_SIZE))
                            .clicked()
                        {};
                    });
                });
            }
        });
    }
}

impl YanuApp {
    fn top_bar(&mut self, egui_ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(egui_ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ui.close_menu();
                            process::exit(0);
                        }
                    });

                    ui.menu_button("Config", |ui| {});
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    egui::warn_if_debug_build(ui);
                    if !cfg!(debug_assertions) {
                        ui.label(
                            RichText::new(env!("CARGO_PKG_VERSION"))
                                .color(egui::Color32::LIGHT_GREEN),
                        );
                    }
                    ui.hyperlink_to("", "https://github.com/nozwock/yanu");
                });
            });
        });
    }
}
