use std::{path::PathBuf, sync::mpsc::TryRecvError, thread, time::Instant};

use common::{
    defines::{APP_CACHE_DIR, APP_CONFIG_DIR, DEFAULT_PRODKEYS_PATH, SWITCH_DIR},
    format::HumanDuration,
    utils::get_fmt_size,
};
use config::{Config, NcaExtractor, NspExtractor};
use eframe::egui;
use egui::RichText;
use egui_modal::Modal;
use eyre::{bail, Result};
use fs_err as fs;
use hac::{
    utils::{formatted_nsp_rename, pack::pack_fs_data, unpack::unpack_nsp, update::update_nsp},
    vfs::{nsp::Nsp, validate_program_id, xci::xci_to_nsps},
};
use tracing::info;

use super::{cross_centered, increase_font_size_by};
use crate::{
    utils::{
        check_keyfile_exists, consume_err, consume_err_or, default_pack_outdir, pick_nsp_file,
    },
    MpscChannel,
};

#[derive(Debug, Default)]
pub struct YanuApp {
    page: Page,
    config: Config,
    timer: Option<Instant>,
    channel: MpscChannel<Message>,

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
    Loading,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
enum ConvertKind {
    #[default]
    Nsp,
}

impl ConvertKind {
    fn reach_from_types(&self) -> &[&'static str] {
        match self {
            ConvertKind::Nsp => &["xci"],
        }
    }
}

#[derive(Debug)]
enum Converted {
    Nsp(Vec<Nsp>),
}

#[derive(Debug)]
enum Message {
    Update(Result<Nsp>),
    Unpack(Result<PathBuf>),
    Pack(Result<Nsp>),
    Convert(Result<Converted>),
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

        Self {
            // TODO: Handle this somehow, maybe show a dialog message and then exit
            config: Config::load().unwrap(),
            ..Default::default()
        }
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.config.clone().store().unwrap();
        info!(config = ?self.config, "Stored Config before exiting...");
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, frame.info().native_pixels_per_point);

        let mut dialog_modal = Modal::new(ctx, "dialog modal");
        dialog_modal.show_dialog();

        show_top_bar(ctx, frame, &dialog_modal, &mut self.config, &self.page);

        if self.page != Page::Loading {
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
        }

        egui::CentralPanel::default().show(ctx, |_ui| match self.page {
            Page::Update => {
                cross_centered("center update", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Base file:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.base_pkg_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    pick_nsp_file(&dialog_modal, Some("Pick a Base file"), |path| {
                                        self.base_pkg_path_buf = path.to_string_lossy().into();
                                    });
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("Update file:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.update_pkg_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    pick_nsp_file(&dialog_modal, Some("Pick an Update file"), |path| {
                                            self.update_pkg_path_buf = path.to_string_lossy().into();
                                    });
                                };
                            });

                            ui.add_space(PADDING);

                            ui.checkbox(&mut self.overwrite_titleid, "Overwrite TitleID");
                            if self.overwrite_titleid {
                            ui.text_edit_singleline(&mut self.pack_title_id_buf)
                                .on_hover_text(
                                    "Check the logs or output for guidance on \n\
                                which TitleID to use if using the wrong one.\n\
                                                                For eg:\n\
                                                                'TitleID mismatch!\n\
                                                                ACI0 TitleID: xxxxxxxxxxxxxxxx'",
                                );
                            }
                        });
                    });

                    ui.add_space(PADDING);

                    ui.vertical_centered(|ui| {
                        if ui
                            .button(RichText::new("Update").size(HEADING_SIZE))
                            .clicked()
                        {
                            self.do_update(&dialog_modal);
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
                                    pick_nsp_file(&dialog_modal, Some("Pick a Base file"), |path| {
                                        self.base_pkg_path_buf = path.to_string_lossy().into();
                                    });
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("Update file:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.update_pkg_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    pick_nsp_file(&dialog_modal, Some("Pick an Update file"), |path| {
                                            self.update_pkg_path_buf = path.to_string_lossy().into();
                                    });
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
                            self.do_unpack(&dialog_modal);
                        };
                    });
                });
            }
            Page::Pack => {
                cross_centered("center pack", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Control NCA:")
                                .on_hover_text("Control NCA is typically around 1MB in size.");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                // TODO: Figure out how to move the focus to the end on demand
                                // let text_edit =
                                //     egui::TextEdit::singleline(&mut self.control_nca_path).show(ui);

                                ui.text_edit_singleline(&mut self.control_nca_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    consume_err_or(
                                        "No file was picked",
                                        &dialog_modal,
                                        rfd::FileDialog::new()
                                            .set_title("Pick a Control NCA file")
                                            .add_filter("NCA", &["nca"])
                                            .pick_file(),
                                        |path| {
                                            info!(?path, size = %get_fmt_size(&path).unwrap_or_default(), "Selected file");
                                            self.control_nca_path_buf = path.to_string_lossy().into();
                                        },
                                    );
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("TitleID:");
                            ui.text_edit_singleline(&mut self.pack_title_id_buf)
                                .on_hover_text(
                                    "Check the logs or output for guidance on \n\
                                which TitleID to use if using the wrong one.\n\
                                                                For eg:\n\
                                                                'TitleID mismatch!\n\
                                                                ACI0 TitleID: xxxxxxxxxxxxxxxx'",
                                );

                            ui.add_space(PADDING);

                            ui.label("RomFS folder:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.romfs_dir_buf);
                                if ui.button("📂 Browse").clicked() {
                                    consume_err_or(
                                        "No folder was picked",
                                        &dialog_modal,
                                        rfd::FileDialog::new()
                                            .set_title("Pick a RomFS folder")
                                            .pick_folder(),
                                        |dir| {
                                            self.romfs_dir_buf = dir.to_string_lossy().into();
                                        },
                                    );
                                };
                            });

                            ui.add_space(PADDING);

                            ui.label("ExeFS folder:");
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.exefs_dir_buf);
                                if ui.button("📂 Browse").clicked() {
                                    consume_err_or(
                                        "No folder was picked",
                                        &dialog_modal,
                                        rfd::FileDialog::new()
                                            .set_title("Pick a ExeFS folder")
                                            .pick_folder(),
                                        |dir| {
                                            self.exefs_dir_buf = dir.to_string_lossy().into();
                                        },
                                    );
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
                            self.do_pack(&dialog_modal);
                        };
                    });
                });
            }
            Page::Convert => {
                cross_centered("center convert", ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            ui.label("Source:").on_hover_text(format!(
                                "Possible Types: {}",
                                self.convert_kind.reach_from_types().join(",")
                            ));
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.text_edit_singleline(&mut self.source_file_path_buf);
                                if ui.button("📂 Browse").clicked() {
                                    consume_err_or(
                                        "No file was picked",
                                        &dialog_modal,
                                        rfd::FileDialog::new().pick_file(),
                                        |path| {
                                            info!(?path, size = %get_fmt_size(&path).unwrap_or_default(), "Picked file");
                                            self.source_file_path_buf = path.to_string_lossy().into();
                                        },
                                    );
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
                            self.do_convert(&dialog_modal);
                        };
                    });
                });
            },
            Page::Loading => {
                cross_centered("center loading", ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(format!("{}", HumanDuration(self.timer.expect("must be set to `Some` before the Loading page").elapsed())));
                        ui.add_space(PADDING * 2.);
                        ui.add(egui::Spinner::default().size(HEADING_SIZE * 2.5));
                    });
                });

                match self.channel.rx.try_recv() {
                    Err(err) if err == TryRecvError::Empty => {}
                    rest => {
                        match rest {
                            Ok(message) => match message {
                                Message::Update(response) => {
                                    self.page = Page::Update;
                                    consume_err(
                                        &dialog_modal,
                                        response,
                                        |patched| {
                                            dialog_modal.open_dialog(
                                                None::<&str>,
                                                Some(format!(
                                                    "Patched file created at:\n'{}'\nTook {}",
                                                    patched.path.display(),
                                                    HumanDuration(
                                                        self.timer.expect("must be set to `Some` before the Loading page").elapsed()
                                                    )
                                                )),
                                                Some(egui_modal::Icon::Success),
                                            );
                                        }
                                    );
                                }
                                Message::Unpack(response) => {
                                    self.page = Page::Unpack;
                                    consume_err(
                                        &dialog_modal,
                                        response,
                                        |outdir| {
                                            dialog_modal.open_dialog(
                                                None::<&str>,
                                                Some(format!(
                                                    "Unpacked to '{}'\nTook {}",
                                                    outdir.display(),
                                                    HumanDuration(
                                                        self.timer.expect("must be set to `Some` before the Loading page").elapsed()
                                                    )
                                                )),
                                                Some(egui_modal::Icon::Success),
                                            );
                                        }
                                    );
                                },
                                Message::Pack(response) => {
                                    self.page = Page::Pack;
                                    consume_err(
                                        &dialog_modal,
                                        response,
                                        |packed| {
                                            dialog_modal.open_dialog(
                                                None::<&str>,
                                                Some(format!(
                                                    "Packed NSP created at '{}'\nTook {}",
                                                    packed.path.display(),
                                                    HumanDuration(
                                                        self.timer.expect("must be set to `Some` before the Loading page").elapsed()
                                                    )
                                                )),
                                                Some(egui_modal::Icon::Success),
                                            );
                                        }
                                    );
                                },
                                Message::Convert(response) => {
                                    self.page = Page::Convert;
                                    // This was manually formatted -_-
                                    consume_err(
                                        &dialog_modal,
                                        response,
                                        |converted| {
                                            match converted {
                                                Converted::Nsp(nsps) => {
                                                    dialog_modal.open_dialog(
                                                        None::<&str>,
                                                        Some(format!(
                                                            "Converted NSPs:\n{}",
                                                            itertools::intersperse(
                                                                nsps.iter()
                                                                    .flat_map(|nsp| nsp.path.file_name())
                                                                    .map(|name| format!("- \"{}\"", name.to_string_lossy())),
                                                                "\n".into()
                                                            )
                                                            .collect::<String>()
                                                        )),
                                                        Some(egui_modal::Icon::Success),
                                                    );
                                                },
                                            }
                                        }
                                    );
                                },
                            }
                            Err(err) => {
                                self.page = Page::default();
                                dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
                            },
                        };

                        // Reset timer
                        self.timer = None;
                    }
                };
            },
        });
    }
}

fn show_top_bar(
    ctx: &egui::Context,
    frame: &mut eframe::Frame,
    dialog_modal: &Modal,
    config: &mut Config,
    page: &Page,
) {
    egui::TopBottomPanel::top("top bar").show(ctx, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.add_enabled_ui(!page.eq(&Page::Loading), |ui| {
                        if ui.button("Import Keyfile").clicked() {
                            ui.close_menu();
                            consume_err(
                                dialog_modal,
                                || -> Result<PathBuf> {
                                    let keyfile_path = rfd::FileDialog::new()
                                        .set_title("Pick a Keyfile")
                                        .add_filter("Keyfile", &["keys"])
                                        .pick_file()
                                        .ok_or_else(|| eyre::eyre!("No Keyfile was picked"))?;
                                    info!(?keyfile_path, "Picked keyfile");
                                    assert!(keyfile_path.is_file());

                                    let dest = DEFAULT_PRODKEYS_PATH.as_path();
                                    fs::create_dir_all(
                                        dest.parent()
                                            .ok_or_else(|| eyre::eyre!("Failed to get parent"))?,
                                    )?;
                                    fs::copy(keyfile_path.as_path(), dest)?;
                                    Ok(keyfile_path)
                                }(),
                                |keyfile_path| {
                                    dialog_modal.open_dialog(
                                        None::<&str>,
                                        Some(format!("Imported '{}'", keyfile_path.display())),
                                        Some(egui_modal::Icon::Success),
                                    );
                                },
                            );
                        }

                        ui.separator();
                        consume_err(
                            dialog_modal,
                            || -> Result<()> {
                                if ui.button("Open Config Folder").clicked() {
                                    ui.close_menu();
                                    opener::open(APP_CONFIG_DIR.as_path())?;
                                }
                                if ui.button("Open Cache Folder").clicked() {
                                    ui.close_menu();
                                    opener::open(APP_CACHE_DIR.as_path())?;
                                }
                                if ui.button("Open Keys Folder").clicked() {
                                    ui.close_menu();
                                    opener::open(SWITCH_DIR.as_path())?;
                                }
                                Ok(())
                            }(),
                            |_| {},
                        );
                    });

                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ui.close_menu();
                        frame.close();
                    }
                });
                ui.add_enabled_ui(!page.eq(&Page::Loading), |ui| {
                    ui.menu_button("Config", |ui| {
                        ui.menu_button("Temp Folder", |ui| {
                            if ui.button("Reset").clicked() {
                                ui.close_menu();
                                config.temp_dir = ".".into();
                                dialog_modal.open_dialog(
                                    None::<&str>,
                                    Some("Resetted Temp folder"),
                                    Some(egui_modal::Icon::Success),
                                );
                            }
                            if ui.button("Pick folder").clicked() {
                                ui.close_menu();
                                consume_err_or(
                                    "No folder was picked",
                                    dialog_modal,
                                    rfd::FileDialog::new()
                                        .set_title("Pick a folder to create Temp folders in")
                                        .pick_folder(),
                                    |dir| {
                                        dialog_modal.open_dialog(
                                            None::<&str>,
                                            Some(format!(
                                                "Set '{}' as the folder to create Temp folders in",
                                                dir.display()
                                            )),
                                            Some(egui_modal::Icon::Success),
                                        );
                                        config.temp_dir = dir;
                                    },
                                );
                            }
                        });
                        ui.menu_button("NSP Extractor", |ui| {
                            ui.radio_value(
                                &mut config.nsp_extractor,
                                NspExtractor::Hactool,
                                "Hactool",
                            );
                            ui.radio_value(
                                &mut config.nsp_extractor,
                                NspExtractor::Hactoolnet,
                                "Hactoolnet",
                            );
                        });
                        ui.menu_button("NCA Extractor", |ui| {
                            ui.radio_value(&mut config.nca_extractor, NcaExtractor::Hac2l, "Hac2l");
                            ui.radio_value(
                                &mut config.nca_extractor,
                                NcaExtractor::Hactoolnet,
                                "Hactoolnet",
                            );
                        });
                    });
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.add_space(PADDING);
                egui::warn_if_debug_build(ui);
                if !cfg!(debug_assertions) {
                    ui.label(
                        RichText::new(env!("CARGO_PKG_VERSION")).color(egui::Color32::LIGHT_GREEN),
                    );
                }
                ui.hyperlink_to(" Github", "https://github.com/nozwock/yanu");
            });
        });
    });
}

impl YanuApp {
    fn do_update(&mut self, dialog_modal: &Modal) {
        if let Err(err) = || -> Result<()> {
            check_keyfile_exists()?;

            self.config.clone().store()?;
            self.timer = Some(Instant::now());

            if self.base_pkg_path_buf.is_empty() || self.update_pkg_path_buf.is_empty() {
                bail!("All fields are required")
            }

            let program_id = if self.overwrite_titleid {
                validate_program_id(&self.overwrite_titleid_buf)?;
                Some(self.overwrite_titleid_buf.clone())
            } else {
                None
            };

            let base_pkg_path = self.base_pkg_path_buf.clone();
            let update_pkg_path = self.update_pkg_path_buf.clone();

            let config = self.config.clone();
            let tx = self.channel.tx.clone();
            thread::spawn(move || {
                tx.send(Message::Update(|| -> Result<Nsp> {
                    let (mut patched, nacp_data, program_id) = update_nsp(
                        &mut Nsp::try_new(base_pkg_path)?,
                        &mut Nsp::try_new(update_pkg_path)?,
                        program_id.as_deref(),
                        default_pack_outdir()?,
                        &config,
                    )?;
                    formatted_nsp_rename(
                        &mut patched.path,
                        &nacp_data,
                        &program_id,
                        concat!("[yanu-", env!("CARGO_PKG_VERSION"), "-patched]"),
                    )?;
                    Ok(patched)
                }()))
                .unwrap();
            });

            self.page = Page::Loading;

            Ok(())
        }() {
            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
        };
    }
    fn do_unpack(&mut self, dialog_modal: &Modal) {
        if let Err(err) = || -> Result<()> {
            check_keyfile_exists()?;

            self.config.clone().store()?;
            self.timer = Some(Instant::now());

            if self.base_pkg_path_buf.is_empty() {
                bail!("Base file field must be set");
            }

            let base_pkg_path = self.base_pkg_path_buf.clone();
            let update_pkg_path = if self.update_pkg_path_buf.is_empty() {
                None
            } else {
                Some(self.update_pkg_path_buf.clone())
            };

            let prefix = if update_pkg_path.is_some() {
                "base+patch."
            } else {
                "base."
            };
            let outdir = tempfile::Builder::new()
                .prefix(prefix)
                .tempdir_in(std::env::current_dir()?)?
                .into_path();

            let config = self.config.clone();
            let tx = self.channel.tx.clone();
            thread::spawn(move || {
                tx.send(Message::Unpack(|| -> Result<PathBuf> {
                    unpack_nsp(
                        &mut Nsp::try_new(base_pkg_path)?,
                        update_pkg_path.and_then(|f| Nsp::try_new(f).ok()).as_mut(),
                        &outdir,
                        &config,
                    )?;
                    Ok(outdir)
                }()))
                .unwrap();
            });

            self.page = Page::Loading;

            Ok(())
        }() {
            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
        }
    }
    fn do_pack(&mut self, dialog_modal: &Modal) {
        if let Err(err) = || -> Result<()> {
            check_keyfile_exists()?;

            self.config.clone().store()?;
            self.timer = Some(Instant::now());

            if self.pack_title_id_buf.is_empty()
                || self.control_nca_path_buf.is_empty()
                || self.romfs_dir_buf.is_empty()
                || self.exefs_dir_buf.is_empty()
            {
                bail!("All fields are required");
            }

            validate_program_id(&self.pack_title_id_buf)?;
            let program_id = self.pack_title_id_buf.clone();

            let control_path = self.control_nca_path_buf.clone();
            let romfs_dir = self.romfs_dir_buf.clone();
            let exefs_dir = self.exefs_dir_buf.clone();
            let outdir = default_pack_outdir()?;

            let config = self.config.clone();
            let tx = self.channel.tx.clone();
            thread::spawn(move || {
                tx.send(Message::Pack(|| -> Result<Nsp> {
                    let (mut patched, nacp_data) = pack_fs_data(
                        control_path,
                        program_id.clone(),
                        romfs_dir,
                        exefs_dir,
                        outdir,
                        &config,
                    )?;
                    formatted_nsp_rename(
                        &mut patched.path,
                        &nacp_data,
                        &program_id,
                        concat!("[yanu-", env!("CARGO_PKG_VERSION"), "-packed]"),
                    )?;
                    Ok(patched)
                }()))
                .unwrap();
            });

            self.page = Page::Loading;

            Ok(())
        }() {
            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
        }
    }
    fn do_convert(&mut self, dialog_modal: &Modal) {
        if let Err(err) = || -> Result<()> {
            check_keyfile_exists()?;

            self.timer = Some(Instant::now());

            let source_path = PathBuf::from(&self.source_file_path_buf);
            let is_convertable = source_path
                .extension()
                .map(|ext| {
                    self.convert_kind
                        .reach_from_types()
                        .iter()
                        .any(|s| *s == ext)
                })
                .unwrap_or_default();

            if !is_convertable {
                bail!(
                    "Not supported conversion '{:?} -> {:?}'",
                    source_path.extension(),
                    self.convert_kind
                )
            }

            let convert_kind = self.convert_kind;
            let outdir = default_pack_outdir()?;
            let tempdir_in = self.config.temp_dir.clone();

            let tx = self.channel.tx.clone();
            thread::spawn(move || {
                tx.send(Message::Convert(|| -> Result<Converted> {
                    let converted = match convert_kind {
                        ConvertKind::Nsp => match source_path.extension() {
                            Some(ext) if ext == "xci" => {
                                Converted::Nsp(xci_to_nsps(source_path, outdir, tempdir_in)?)
                            }
                            Some(_) => bail!("Need to implement"),
                            None => bail!("Non Unicode in the path"),
                        },
                    };
                    Ok(converted)
                }()))
                .unwrap();
            });

            self.page = Page::Loading;

            Ok(())
        }() {
            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error));
        }
    }
}
