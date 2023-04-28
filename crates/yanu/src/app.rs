use eframe::egui;

#[derive(Debug, Default)]
pub struct YanuApp {
    page: Page,
}

#[derive(Debug, Default, PartialEq)]
enum Page {
    #[default]
    Update,
    Pack,
    Repack,
    Convert,
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

        Default::default()
    }
}

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

        egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {});

        egui::SidePanel::left("actions_panel")
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::new([true, true])
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.selectable_value(&mut self.page, Page::Update, "Update");
                        ui.selectable_value(&mut self.page, Page::Pack, "Pack");
                        ui.selectable_value(&mut self.page, Page::Repack, "Repack");
                        ui.selectable_value(&mut self.page, Page::Convert, "Convert");
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.page {
            Page::Update => {}
            Page::Pack => {}
            Page::Repack => {}
            Page::Convert => {}
        });
    }
}
