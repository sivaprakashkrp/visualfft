use eframe::egui::{
    self, CentralPanel, ComboBox, Context, FontFamily, FontId, ScrollArea, TextStyle, TopBottomPanel, Ui
};

#[derive(Default)]
struct App {

}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        todo!("Render input field, display field and graphs");
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_resizable(true)
            .with_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    let _ = eframe::run_native("VisualFFT", options, Box::new(|_cc| Ok(Box::<App>::default())));
    println!("App Closed");
}
