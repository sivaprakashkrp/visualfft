use eframe::egui::{
    self, CentralPanel, Context, FontFamily, FontId, ScrollArea, TextStyle, TopBottomPanel, Ui
};

#[derive(Default)]
struct App {
    input_eqn: String,
    
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        set_styles(ctx);
        show_top_bar(ctx);
        CentralPanel::default().show(ctx, |ui| {
            self.show_expr_input(ui);
        });
    }
}

impl App {
    fn show_expr_input(&mut self, ui: &mut Ui) {
        ui.vertical_centered_justified(|ui| {
            ui.horizontal(|ui| {
                ui.label("Input Expression: ");
                ui.add_sized(
                    egui::vec2(500.0, 100.0), 
                    egui::TextEdit::singleline(&mut self.input_eqn)       
                );
            });
            ui.horizontal(|ui| {
                if ui.button("Apply FFT").clicked() {
                    // todo!("Apply FFT");
                }
                if ui.button("Clear").clicked() {
                    self.input_eqn.clear();
                }
            })
        });
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
}

fn set_styles(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(25.0, FontFamily::Monospace)),
        (TextStyle::Body, FontId::new(16.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(16.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(14.0, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
}

fn show_top_bar(ctx: &Context) {
    TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("About").clicked() {
                    // todo!("Render About Dialog Box")
                }
                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            })
        })
    });
}
