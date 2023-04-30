use std::process;

use common::utils::get_size_as_string;
use eframe::egui;
use egui::RichText;
use egui_modal::Modal;
use tracing::info;

use super::{cross_centered, increase_font_size_by};
use crate::utils::{pick_nca_file, pick_nsp_file};

#[derive(Debug, Default)]
pub struct YanuApp {
    page: Page,

    // need channel for modal windows...
    // channel_rx: Option<...>

    // Update Page
    overwrite_titleid: bool,
    overwrite_titleid_buf: String,

    // Update/Unpack Page
    base_pkg_path_buf: String,
    update_pkg_path_buf: String,

    // Pack Page
    control_nca_path_buf: String,
    pack_title_id_buf: String,
    romfs_dir_buf: String,
    exefs_dir_buf: String,

    // Convert Page
    source_file_path_buf: String,
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

#[derive(Debug, PartialEq)]
enum Message {}

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
const BODY_SIZE: f32 = 12.5; // 1.2x of default
const PADDING: f32 = BODY_SIZE * 0.5; // 0.5x of default Body size

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

        let mut dialog_modal = Modal::new(ctx, "dialog modal");
        dialog_modal.show_dialog();

        egui::SidePanel::left("options panel")
            .resizable(false)
            .default_width(100.)
            .show(ctx, |ui| {
                ui.add_space(PADDING * 0.8);
                ui.vertical_centered(|ui| {
                    ui.heading("Options");
                });

                ui.separator();
                ui.add_space(PADDING * 0.6);

                egui::ScrollArea::new([true, true])
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.spacing_mut().item_spacing.y *= 1.5;
                            ui.selectable_value(&mut self.page, Page::Update, "Update");
                            ui.selectable_value(&mut self.page, Page::Unpack, "Unpack");
                            ui.selectable_value(&mut self.page, Page::Pack, "Pack");
                            ui.selectable_value(&mut self.page, Page::Convert, "Convert");
                        });
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Update => {
                cross_centered("center update", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Base file:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.base_pkg_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match pick_nsp_file(Some("Pick a Base file")) {
                                        Ok(path) => {
                                            self.base_pkg_path_buf = path.to_string_lossy().into();
                                        }
                                        Err(err) => {
                                            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("Update file:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.update_pkg_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match pick_nsp_file(Some("Pick an Update file")) {
                                        Ok(path) => {
                                            self.update_pkg_path_buf = path.to_string_lossy().into();
                                        }
                                        Err(err) => {
                                            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });

                            ui.add_space(PADDING);

                            ui.checkbox(&mut self.overwrite_titleid, "Overwrite TitleID");
                            if self.overwrite_titleid {
                                ui.text_edit_singleline(&mut self.overwrite_titleid_buf);
                            }
                        });
                    });

                    ui.add_space(PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Update").size(HEADING_SIZE))
                            .clicked()
                        {
                            todo!("validate TitleID if set to overwrite and use `update_nsp`")
                        };
                    });
                });
            }
            Page::Unpack => {
                cross_centered("center unpack", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Base file:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.base_pkg_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match pick_nsp_file(Some("Pick a Base file")) {
                                        Ok(path) => {
                                            self.base_pkg_path_buf = path.to_string_lossy().into();
                                        }
                                        Err(err) => {
                                            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("Update file:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.update_pkg_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match pick_nsp_file(Some("Pick an Update file")) {
                                        Ok(path) => {
                                            self.update_pkg_path_buf = path.to_string_lossy().into();
                                        }
                                        Err(err) => {
                                            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });
                        });
                    });

                    ui.add_space(PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Unpack").size(HEADING_SIZE))
                            .clicked()
                        {
                            todo!("use `unpack_nsp`")
                        };
                    });
                });
            }
            Page::Pack => {
                cross_centered("center pack", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Control NCA:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                // let text_edit =
                                //     egui::TextEdit::singleline(&mut self.control_nca_path).show(ui);

                                // TODO: Figure out how to move the focus to the end on demand
                                ui.text_edit_singleline(&mut self.control_nca_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match pick_nca_file(Some("Pick a Control NCA file")) {
                                        Ok(path) => {
                                            self.control_nca_path_buf = path.to_string_lossy().into();
                                        }
                                        Err(err) => {
                                            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("TitleID:");
                            ui.text_edit_singleline(&mut self.pack_title_id_buf);

                            ui.add_space(PADDING);

                            ui.label("RomFS folder:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.romfs_dir_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match rfd::FileDialog::new()
                                        .set_title("Pick a RomFS folder")
                                        .pick_folder()
                                    {
                                        Some(dir) => {
                                            self.romfs_dir_buf = dir.to_string_lossy().into();
                                        }
                                        None => {
                                            dialog_modal.open_dialog(None::<&str>, Some("No folder was selected"), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("ExeFS folder:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.exefs_dir_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match rfd::FileDialog::new()
                                        .set_title("Pick a ExeFS folder")
                                        .pick_folder()
                                    {
                                        Some(dir) => {
                                            self.exefs_dir_buf = dir.to_string_lossy().into();
                                        }
                                        None => {
                                            dialog_modal.open_dialog(None::<&str>, Some("No folder was selected"), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });
                        });
                    });

                    ui.add_space(PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Pack").size(HEADING_SIZE))
                            .clicked()
                        {
                            // TODO: Check ContentType and make sure selected NCA is of Control type -> this will be done in `pack_fs_data`, so no need.
                            todo!("validate TitleID and use `pack_fs_data`")
                        };
                    });
                });
            }
            Page::Convert => {
                cross_centered("center convert", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Source:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.source_file_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    match rfd::FileDialog::new().pick_file() {
                                        Some(path) => {
                                            info!(?path, size = %get_size_as_string(&path).unwrap_or_default(), "Selected file");
                                            self.source_file_path_buf = path.to_string_lossy().into();
                                        }
                                        None => {
                                            dialog_modal.open_dialog(None::<&str>, Some("No file was selected"), Some(egui_modal::Icon::Error));
                                        },
                                    }
                                };
                            });

                            ui.add_space(PADDING);

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

                    ui.add_space(PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Convert").size(HEADING_SIZE))
                            .clicked()
                        {
                            todo!("Check whether source can be converted to target type and then use `xci_to_nsps`")
                        };
                    });
                });
            }
        });
    }
}

impl YanuApp {
    fn top_bar(&mut self, egui_ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top bar").show(egui_ctx, |ui| {
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
                    ui.add_space(PADDING);
                    egui::warn_if_debug_build(ui);
                    if !cfg!(debug_assertions) {
                        ui.label(
                            RichText::new(env!("CARGO_PKG_VERSION"))
                                .color(egui::Color32::LIGHT_GREEN),
                        );
                    }
                    ui.hyperlink_to(" Github", "https://github.com/nozwock/yanu");
                });
            });
        });
    }
}
