mod app;
mod file_processing;
mod file_saver;
mod network;
mod syntax_highlighter;


mod proxy;

use app::SiteMapperApp;
use eframe::egui;

fn hacker_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(200, 200, 200)); // Silver
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 30); // Dark grey
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 50, 50);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(90, 90, 90);
    visuals.selection.bg_fill = egui::Color32::from_rgb(200, 50, 50); // Red
    visuals.hyperlink_color = egui::Color32::from_rgb(255, 255, 0); // Yellow

    visuals.widgets.noninteractive.rounding = egui::Rounding::same(5.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(5.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(5.0);
    visuals.widgets.active.rounding = egui::Rounding::same(5.0);
    visuals.window_rounding = egui::Rounding::same(5.0);

    visuals
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("maya.vi"),
        ..Default::default()
    };
    eframe::run_native(
        "sitemapper",
        options,
        Box::new(|cc: &eframe::CreationContext| {
            cc.egui_ctx.set_visuals(hacker_visuals());

            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.iter_mut().for_each(|(_, font_data)| {
                font_data.tweak.scale = 1.2;
            });
            cc.egui_ctx.set_fonts(fonts);
            
            Box::<SiteMapperApp>::default()
        }),
    )
}