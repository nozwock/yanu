use std::process;

use eframe::egui;
use egui::RichText;

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

        setup_fonts(&cc.egui_ctx);

        Default::default()
    }
}

const VERTICAL_PADDING: f32 = 7.;

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
                egui::Window::new("update_window")
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .title_bar(false)
                    .frame(egui::Frame::none())
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.label("Base Package:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.base_pkg_path);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );

                                ui.label("Update Package:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.update_pkg_path);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );
                            });
                        });

                        ui.add_space(VERTICAL_PADDING);

                        ui.vertical_centered(|ui| {
                            if ui.button(RichText::new("Update").size(24.)).clicked() {};
                        });
                    });
            }
            Page::Unpack => {
                egui::Window::new("unpack_window")
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .title_bar(false)
                    .frame(egui::Frame::none())
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.label("Base Package:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.base_pkg_path);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );

                                ui.label("Update Package:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.update_pkg_path);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );
                            });
                        });

                        ui.add_space(VERTICAL_PADDING);

                        ui.vertical_centered(|ui| {
                            if ui.button(RichText::new("Unpack").size(24.)).clicked() {};
                        })
                    });
            }
            Page::Pack => {
                egui::Window::new("pack_window")
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .title_bar(false)
                    .frame(egui::Frame::none())
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.label("Control NCA:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.control_nca_path);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );

                                ui.label("TitleID:");
                                ui.text_edit_singleline(&mut self.pack_title_id);

                                ui.label("RomFS directory:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.romfs_dir);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );

                                ui.label("ExeFS directory:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.exefs_dir);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );
                            });
                        });

                        ui.add_space(VERTICAL_PADDING);

                        ui.vertical_centered(|ui| {
                            if ui.button(RichText::new("Pack").size(24.)).clicked() {};
                        })
                    });
            }
            Page::Convert => {
                egui::Window::new("convert_window")
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .title_bar(false)
                    .frame(egui::Frame::none())
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.group(|ui| {
                                ui.label("Source:");
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        ui.text_edit_singleline(&mut self.control_nca_path);
                                        if ui.button("📂 Browse").clicked() {};
                                    },
                                );

                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
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
                                    },
                                );
                            });
                        });

                        ui.add_space(VERTICAL_PADDING);

                        ui.vertical_centered(|ui| {
                            if ui.button(RichText::new("Convert").size(24.)).clicked() {};
                        })
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

fn setup_fonts(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    // * Other ways to increase font size but is set for every element
    // style.override_font_id = Some(egui::FontId::proportional(24.));
    // for (_text_style, font_id) in style.text_styles.iter_mut() {
    //     font_id.size = 16.;
    // }

    style
        .text_styles
        .get_mut(&egui::TextStyle::Body)
        .unwrap()
        .size = 15.;
    style
        .text_styles
        .get_mut(&egui::TextStyle::Heading)
        .unwrap()
        .size = 24.;
    style
        .text_styles
        .get_mut(&egui::TextStyle::Button)
        .unwrap()
        .size = 15.;
    ctx.set_style(style);
}
