use clap::Parser;
use csv::ReaderBuilder;
use eframe::egui::{
    self, CentralPanel, Color32, ComboBox, Context, FontFamily, FontId, Grid, TextStyle,
    TopBottomPanel, Ui,
};
use egui_plot::{Legend, Line, MarkerShape, Plot, Points};
use rustfft::{FftPlanner, num_complex::Complex32};
use serde::Deserialize;
use std::error::Error;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    version,
    author,
    about = "A simple FFT visualization tool built with Rust and egui.",
    long_about = "A GUI Tool that can solve and visualize the results of Fast Fourier Transform (FFT) for a given input sequence. Also supports a CLI mode to process CSV files with FFT configurations and output results in the GUI.",
    help_template = "{bin} {version}\nDeveloped By: {author}\n\n{about}\n\nUsage:\n\t{usage}\n\n{all-args}",
    author = "Sivaprakash P"
)]
struct CliArgs {
    #[arg(
        short = 'c',
        long = "csv-file",
        value_name = "FILE",
        help = "Path to CSV file with column: InputSequence"
    )]
    csv_file: Option<PathBuf>,

    #[arg(
        short = 'i',
        long = "sampling-interval",
        value_name = "DT",
        help = "Sampling interval dt (required in CLI mode if --sampling-frequency is not provided)"
    )]
    sampling_interval: Option<f64>,

    #[arg(
        short = 'f',
        long = "sampling-frequency",
        value_name = "FS",
        help = "Sampling frequency fs (required in CLI mode if --sampling-interval is not provided)"
    )]
    sampling_frequency: Option<f64>,

    #[arg(
        short = 'd',
        long = "direction",
        default_value = "forward",
        value_name = "DIRECTION",
        help = "Transform direction: forward or inverse"
    )]
    direction: String,

    #[arg(short = 'p', long = "preview", default_value_t = 12, value_name = "ROWS", help = "Number of rows to preview")]
    preview: usize,
}

#[derive(Debug, Deserialize)]
struct CsvRow {
    #[serde(rename = "InputSequence", alias = "input_sequence")]
    input_sequence: String,
}

#[derive(Clone, Copy)]
struct SamplingConfig {
    dt: f64,
    fs: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransformDirection {
    Forward,
    Inverse,
}

impl Default for TransformDirection {
    fn default() -> Self {
        Self::Forward
    }
}

impl TransformDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "Forward",
            Self::Inverse => "Inverse",
        }
    }

    fn parse(input: &str) -> Option<Self> {
        let value = input.trim().to_ascii_lowercase();
        match value.as_str() {
            "forward" | "fwd" | "fft" => Some(Self::Forward),
            "inverse" | "inv" | "ifft" => Some(Self::Inverse),
            _ => None,
        }
    }
}

struct FftConfig {
    input_samples: Vec<Complex32>,
    sampling: SamplingConfig,
    direction: TransformDirection,
}

#[derive(Default)]
struct App {
    input_sequence_text: String,
    sampling_interval_input: String,
    sampling_frequency_input: String,
    direction: TransformDirection,
    status_message: String,
    fft_result: Option<FftResult>,
    focused_plot_id: Option<String>,
}

#[derive(Default, Clone)]
struct FftResult {
    time_division: f64,
    frequency_division: f64,
    input_real_points: Vec<[f64; 2]>,
    input_imag_points: Vec<[f64; 2]>,
    real_points: Vec<[f64; 2]>,
    imag_points: Vec<[f64; 2]>,
    magnitude_points: Vec<[f64; 2]>,
    phase_points: Vec<[f64; 2]>,
}

#[derive(Clone, Copy)]
enum PlotRenderStyle {
    Line,
    StemArrow,
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
    fn new() -> Self {
        Self {
            input_sequence_text: "1, 0, -1, 0, 1, 0, -1, 0".to_string(),
            sampling_interval_input: "0.001".to_string(),
            sampling_frequency_input: "".to_string(),
            direction: TransformDirection::Forward,
            status_message: "Enter data and click Apply FFT.".to_string(),
            fft_result: None,
            focused_plot_id: None,
        }
    }

    fn show_expr_input(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.heading("FFT Parameters");
            ui.add_space(8.0);

            ui.label(
                "Input sequence (comma/space separated). Complex values supported, e.g. 1+2i, -3i, 4.",
            );
            ui.add_sized(
                egui::vec2(ui.available_width(), 90.0),
                egui::TextEdit::multiline(&mut self.input_sequence_text),
            );

            ui.add_space(8.0);
            Grid::new("fft_param_grid").num_columns(2).show(ui, |ui| {
                ui.label("Sampling interval (dt):");
                ui.add_sized(
                    egui::vec2(120.0, 24.0),
                    egui::TextEdit::singleline(&mut self.sampling_interval_input)
                        .hint_text("Optional if fs given"),
                );
                ui.end_row();

                ui.label("Sampling frequency (fs):");
                ui.add_sized(
                    egui::vec2(120.0, 24.0),
                    egui::TextEdit::singleline(&mut self.sampling_frequency_input)
                        .hint_text("Optional if dt given"),
                );
                ui.end_row();

                ui.label("Direction:");
                ComboBox::from_id_salt("direction_combo")
                    .selected_text(self.direction.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.direction,
                            TransformDirection::Forward,
                            "Forward (time -> frequency)",
                        );
                        ui.selectable_value(
                            &mut self.direction,
                            TransformDirection::Inverse,
                            "Inverse (frequency -> time)",
                        );
                    });
                ui.end_row();
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Apply FFT").clicked() {
                    self.apply_fft();
                }

                if ui.button("Clear").clicked() {
                    self.clear_all();
                }

                ui.colored_label(Color32::LIGHT_BLUE, &self.status_message);
            });

            if let Some(result) = self.fft_result.clone() {
                ui.separator();
                ui.heading("Output");
                ui.label(format!(
                    "Time division (dt): {:.6} s | Frequency division (df): {:.6} Hz",
                    result.time_division, result.frequency_division
                ));
                ui.add_space(4.0);

                let plot_area_size = ui.available_size();
                if plot_area_size.y > 0.0 {
                    ui.allocate_ui(plot_area_size, |plot_ui| {
                        self.draw_responsive_plots(plot_ui, &result);
                    });
                }
            }
        });
    }

    fn apply_fft(&mut self) {
        let input_samples = match parse_sequence_input(&self.input_sequence_text) {
            Ok(values) => values,
            Err(message) => {
                self.status_message = message;
                self.fft_result = None;
                return;
            }
        };

        let n = input_samples.len();
        if n < 2 {
            self.status_message = "Need at least 2 parsed samples for FFT.".to_string();
            self.fft_result = None;
            return;
        }

        let sampling = match resolve_sampling(
            &self.sampling_interval_input,
            &self.sampling_frequency_input,
        ) {
            Ok(value) => value,
            Err(message) => {
                self.status_message = message;
                self.fft_result = None;
                return;
            }
        };

        let config = FftConfig {
            input_samples,
            sampling,
            direction: self.direction,
        };

        match compute_fft(&config) {
            Ok(result) => {
                self.fft_result = Some(result);
                if n.is_power_of_two() {
                    self.status_message = "FFT applied successfully.".to_string();
                } else {
                    self.status_message =
                        "FFT applied successfully. Note: N is not a power of 2.".to_string();
                }
            }
            Err(message) => {
                self.fft_result = None;
                self.status_message = message;
            }
        }
    }

    fn clear_all(&mut self) {
        self.input_sequence_text = "1, 0, -1, 0, 1, 0, -1, 0".to_string();
        self.sampling_interval_input = "0.001".to_string();
        self.sampling_frequency_input.clear();
        self.direction = TransformDirection::Forward;
        self.fft_result = None;
        self.focused_plot_id = None;
        self.status_message = "Inputs and plots cleared.".to_string();
    }

    fn draw_responsive_plots(&mut self, ui: &mut Ui, result: &FftResult) {
        let mut plot_specs: Vec<(String, String, Color32, PlotRenderStyle, &[[f64; 2]])> =
            Vec::new();

        plot_specs.push((
            "Input Real Component".to_string(),
            "input_real".to_string(),
            Color32::LIGHT_BLUE,
            PlotRenderStyle::Line,
            &result.input_real_points,
        ));
        plot_specs.push((
            "Input Imag Component".to_string(),
            "input_imag".to_string(),
            Color32::from_rgb(0x9A, 0xE6, 0xB4),
            PlotRenderStyle::Line,
            &result.input_imag_points,
        ));

        plot_specs.push((
            "FFT Real Component".to_string(),
            "fft_real".to_string(),
            Color32::LIGHT_GREEN,
            PlotRenderStyle::Line,
            &result.real_points,
        ));
        plot_specs.push((
            "FFT Imaginary Component".to_string(),
            "fft_imag".to_string(),
            Color32::LIGHT_RED,
            PlotRenderStyle::Line,
            &result.imag_points,
        ));
        plot_specs.push((
            "FFT Magnitude".to_string(),
            "fft_magnitude".to_string(),
            Color32::LIGHT_YELLOW,
            PlotRenderStyle::StemArrow,
            &result.magnitude_points,
        ));
        plot_specs.push((
            "FFT Phase (radians)".to_string(),
            "fft_phase".to_string(),
            Color32::KHAKI,
            PlotRenderStyle::StemArrow,
            &result.phase_points,
        ));

        let columns = if ui.available_width() >= 1200.0 {
            3
        } else if ui.available_width() >= 760.0 {
            2
        } else {
            1
        };

        if let Some(focused_id) = &self.focused_plot_id {
            let focused_spec = plot_specs
                .iter()
                .find(|(_, id, _, _, _)| id == focused_id)
                .map(|(title, id, color, style, points)| {
                    (title.clone(), id.clone(), *color, *style, *points)
                });

            if let Some((title, id, color, style, points)) = focused_spec {
                ui.horizontal(|ui| {
                    ui.strong(format!("Focused: {title}"));
                    if ui.button("Back to all graphs").clicked() {
                        self.focused_plot_id = None;
                    }
                });
                ui.add_space(4.0);

                let focused_plot_height = ui.available_height().max(220.0);
                if Self::draw_component_plot(
                    ui,
                    &title,
                    &id,
                    points,
                    color,
                    style,
                    focused_plot_height,
                ) {
                    self.focused_plot_id = None;
                }

                return;
            }

            self.focused_plot_id = None;
        }

        let total_rows = (plot_specs.len() + columns - 1) / columns;
        let row_spacing = 8.0;
        let available_height = ui.available_height();
        let total_spacing = row_spacing * (total_rows.saturating_sub(1) as f32);
        let per_plot_height = ((available_height - total_spacing) / total_rows as f32).max(140.0);
        let mut requested_focus_id: Option<String> = None;

        for row in plot_specs.chunks(columns) {
            ui.columns(columns, |column_uis| {
                for (column_index, (title, id, color, render_style, points)) in
                    row.iter().enumerate()
                {
                    let was_double_tapped = Self::draw_component_plot(
                        &mut column_uis[column_index],
                        title,
                        id,
                        points,
                        *color,
                        *render_style,
                        per_plot_height,
                    );

                    if was_double_tapped {
                        requested_focus_id = Some(id.clone());
                    }
                }
            });
            ui.add_space(8.0);
        }

        if let Some(id) = requested_focus_id {
            self.focused_plot_id = Some(id);
        }
    }

    fn draw_component_plot(
        ui: &mut Ui,
        title: &str,
        id: &str,
        points: &[[f64; 2]],
        color: Color32,
        render_style: PlotRenderStyle,
        plot_height: f32,
    ) -> bool {
        let plot_response = Plot::new(id)
            .height(plot_height)
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                match render_style {
                    PlotRenderStyle::Line => {
                        let line = Line::new(title, points.to_vec()).color(color);
                        plot_ui.line(line);
                    }
                    PlotRenderStyle::StemArrow => {
                        let mut peaks = Vec::with_capacity(points.len());

                        for point in points {
                            peaks.push(*point);
                        }
                        let arrow_points = Points::new(format!("{title} Arrow"), peaks)
                            .shape(MarkerShape::Up)
                            .radius(6.0)
                            .color(color);

                        for (index, point) in points.iter().enumerate() {
                            let stack_name = if index == 0 {
                                format!("{title} Stacks")
                            } else {
                                String::new()
                            };

                            let stack = Line::new(stack_name, vec![[point[0], 0.0], *point])
                                .color(color)
                                .width(2.0);
                            plot_ui.line(stack);
                        }

                        plot_ui.points(arrow_points);
                    }
                }
            });

        plot_response.response.double_clicked()
    }
}

fn main() {
    let cli = CliArgs::parse();
    if let Some(csv_path) = cli.csv_file.as_ref() {
        if let Err(err) = run_cli(
            csv_path,
            cli.sampling_interval,
            cli.sampling_frequency,
            &cli.direction,
            cli.preview,
        ) {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
        return;
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_resizable(true)
            .with_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    let _ = eframe::run_native("VisualFFT", options, Box::new(|_cc| Ok(Box::new(App::new()))));
}

fn run_cli(
    csv_path: &PathBuf,
    sampling_interval: Option<f64>,
    sampling_frequency: Option<f64>,
    direction_input: &str,
    preview_rows: usize,
) -> Result<(), Box<dyn Error>> {
    let rows = load_configs_from_csv(csv_path)?;
    if rows.is_empty() {
        return Err("CSV file contains no rows".into());
    }

    let sampling = resolve_sampling_from_options(sampling_interval, sampling_frequency)
        .map_err(|message| -> Box<dyn Error> { message.into() })?;
    let direction = TransformDirection::parse(direction_input)
        .ok_or_else(|| -> Box<dyn Error> { "--direction must be forward or inverse".into() })?;

    for (idx, row) in rows.iter().enumerate() {
        let config = config_from_csv_row(row, sampling, direction)
            .map_err(|message| -> Box<dyn Error> { format!("Row {}: {message}", idx + 2).into() })?;

        let result = compute_fft(&config)
            .map_err(|message| -> Box<dyn Error> { format!("Row {}: {message}", idx + 2).into() })?;

        print_cli_report(idx + 1, row, &config, &result, preview_rows);
    }

    Ok(())
}

fn load_configs_from_csv(csv_path: &PathBuf) -> Result<Vec<CsvRow>, Box<dyn Error>> {
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_path(csv_path)?;
    let mut rows = Vec::new();

    for row in reader.deserialize() {
        rows.push(row?);
    }

    Ok(rows)
}

fn config_from_csv_row(
    row: &CsvRow,
    sampling: SamplingConfig,
    direction: TransformDirection,
) -> Result<FftConfig, String> {
    let input_samples = parse_sequence_input(&row.input_sequence)?;

    if input_samples.len() < 2 {
        return Err("Need at least 2 parsed samples for FFT.".to_string());
    }

    Ok(FftConfig {
        input_samples,
        sampling,
        direction,
    })
}

fn compute_fft(config: &FftConfig) -> Result<FftResult, String> {
    if config.input_samples.is_empty() {
        return Err("Input sequence cannot be empty.".to_string());
    }

    let n = config.input_samples.len();
    if n < 2 {
        return Err("N must be at least 2.".to_string());
    }

    let mut working = config.input_samples.clone();
    let original_for_plot = working.clone();

    let dt = config.sampling.dt;
    let fs = config.sampling.fs;
    let df = fs / n as f64;

    let mut planner = FftPlanner::<f32>::new();
    match config.direction {
        TransformDirection::Forward => {
            let fft = planner.plan_fft_forward(n);
            fft.process(&mut working);
        }
        TransformDirection::Inverse => {
            let fft = planner.plan_fft_inverse(n);
            fft.process(&mut working);
            let scale = 1.0 / n as f32;
            for value in &mut working {
                value.re *= scale;
                value.im *= scale;
            }
        }
    }

    let mut result = FftResult::default();
    result.time_division = dt;
    result.frequency_division = df;
    result.input_real_points.reserve(n);
    result.input_imag_points.reserve(n);
    result.real_points.reserve(n);
    result.imag_points.reserve(n);
    result.magnitude_points.reserve(n);
    result.phase_points.reserve(n);

    for (k, value) in original_for_plot.iter().enumerate() {
        let t = k as f64 * dt;
        result.input_real_points.push([t, value.re as f64]);
        result.input_imag_points.push([t, value.im as f64]);
    }

    for (k, value) in working.iter().enumerate() {
        let freq = k as f64 * df;
        let re = value.re as f64;
        let im = value.im as f64;
        let magnitude = (re * re + im * im).sqrt();
        let phase = im.atan2(re);

        result.real_points.push([freq, re]);
        result.imag_points.push([freq, im]);
        result.magnitude_points.push([freq, magnitude]);
        result.phase_points.push([freq, phase]);
    }

    Ok(result)
}

fn parse_sequence_input(raw: &str) -> Result<Vec<Complex32>, String> {
    let mut values = Vec::new();
    for token in raw
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|token| !token.trim().is_empty())
    {
        values.push(parse_complex_token(token)?);
    }

    if values.is_empty() {
        return Err("Input sequence cannot be empty.".to_string());
    }

    Ok(values)
}

fn parse_complex_token(token: &str) -> Result<Complex32, String> {
    let normalized = token.trim().replace('I', "i").replace('j', "i").replace('J', "i");

    if !normalized.contains('i') {
        let re = normalized
            .parse::<f32>()
            .map_err(|_| format!("Invalid numeric token '{token}'"))?;
        return Ok(Complex32 { re, im: 0.0 });
    }

    let terms = split_signed_terms(&normalized);
    if terms.is_empty() {
        return Err(format!("Invalid complex token '{token}'"));
    }

    let mut re = 0.0f32;
    let mut im = 0.0f32;

    for term in terms {
        if let Some(imag_part) = term.strip_suffix('i') {
            let coeff = if imag_part.is_empty() || imag_part == "+" {
                1.0
            } else if imag_part == "-" {
                -1.0
            } else {
                imag_part
                    .parse::<f32>()
                    .map_err(|_| format!("Invalid imaginary term '{term}' in '{token}'"))?
            };
            im += coeff;
        } else {
            re += term
                .parse::<f32>()
                .map_err(|_| format!("Invalid real term '{term}' in '{token}'"))?;
        }
    }

    Ok(Complex32 { re, im })
}

fn split_signed_terms(input: &str) -> Vec<&str> {
    let mut terms = Vec::new();
    let mut start = 0;
    for (idx, ch) in input.char_indices().skip(1) {
        if ch == '+' || ch == '-' {
            let term = input[start..idx].trim();
            if !term.is_empty() {
                terms.push(term);
            }
            start = idx;
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        terms.push(tail);
    }
    terms
}

fn resolve_sampling(dt_input: &str, fs_input: &str) -> Result<SamplingConfig, String> {
    let parsed_dt = if dt_input.trim().is_empty() {
        None
    } else {
        Some(
            dt_input
                .trim()
                .parse::<f64>()
                .map_err(|_| "Invalid sampling interval (dt).".to_string())?,
        )
    };

    let parsed_fs = if fs_input.trim().is_empty() {
        None
    } else {
        Some(
            fs_input
                .trim()
                .parse::<f64>()
                .map_err(|_| "Invalid sampling frequency (fs).".to_string())?,
        )
    };

    match (parsed_dt, parsed_fs) {
        (Some(dt), Some(fs)) => {
            if dt <= 0.0 || fs <= 0.0 {
                return Err("Sampling values must be greater than 0.".to_string());
            }
            Ok(SamplingConfig { dt, fs })
        }
        (Some(dt), None) => {
            if dt <= 0.0 {
                return Err("Sampling interval must be greater than 0.".to_string());
            }
            Ok(SamplingConfig { dt, fs: 1.0 / dt })
        }
        (None, Some(fs)) => {
            if fs <= 0.0 {
                return Err("Sampling frequency must be greater than 0.".to_string());
            }
            Ok(SamplingConfig { dt: 1.0 / fs, fs })
        }
        (None, None) => Err("Provide either sampling interval (dt) or sampling frequency (fs).".to_string()),
    }
}

fn resolve_sampling_from_options(
    sampling_interval: Option<f64>,
    sampling_frequency: Option<f64>,
) -> Result<SamplingConfig, String> {
    let dt_text = sampling_interval.map(|value| value.to_string()).unwrap_or_default();
    let fs_text = sampling_frequency.map(|value| value.to_string()).unwrap_or_default();
    resolve_sampling(&dt_text, &fs_text)
}

fn print_cli_report(
    record_index: usize,
    row: &CsvRow,
    config: &FftConfig,
    result: &FftResult,
    preview_rows: usize,
) {
    let preview = preview_rows.max(1).min(result.magnitude_points.len());

    println!("VisualFFT CLI mode - record {record_index}");
    println!("Direction: {}", config.direction.as_str());
    println!("N: {}", config.input_samples.len());
    println!("Input tokens: {}", config.input_samples.len());
    println!(
        "Sampling interval (dt): {:.9} | Sampling frequency (fs): {:.9}",
        config.sampling.dt, config.sampling.fs
    );
    println!("Frequency division (df): {:.9}", result.frequency_division);
    if !config.input_samples.len().is_power_of_two() {
        println!("Note: N is not a power of 2.");
    }
    println!("CSV source: InputSequence={}", row.input_sequence);
    println!();

    println!("Input sequence preview (first {preview} points)");
    println!("Idx | Time | Real | Imag");
    for i in 0..preview {
        println!(
            "{:>3} | {:>8.5} | {:>8.5} | {:>8.5}",
            i,
            result.input_real_points[i][0],
            result.input_real_points[i][1],
            result.input_imag_points[i][1]
        );
    }
    println!();

    println!("Transform output preview (first {preview} bins)");
    println!("Bin | Freq | Real | Imag | Magnitude | Phase");
    for i in 0..preview {
        println!(
            "{:>3} | {:>8.5} | {:>8.5} | {:>8.5} | {:>9.5} | {:>9.5}",
            i,
            result.real_points[i][0],
            result.real_points[i][1],
            result.imag_points[i][1],
            result.magnitude_points[i][1],
            result.phase_points[i][1]
        );
    }
    println!();
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
        egui::MenuBar::new().ui(ui, |ui| {
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