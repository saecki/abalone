use abalone::{Abalone, Color};
use eframe::NativeOptions;
use egui::{CentralPanel, Color32, Frame, Vec2};

fn main() {
    let native_options = NativeOptions {
        follow_system_theme: true,
        ..Default::default()
    };
    eframe::run_native(
        "abalone",
        native_options,
        Box::new(|_cc| Box::new(AbaloneApp::new())),
    )
    .expect("error running app");
}

struct AbaloneApp {
    game: Abalone,
}

impl AbaloneApp {
    fn new() -> Self {
        Self {
            game: Abalone::new(),
        }
    }
}

impl eframe::App for AbaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let available_size = ui.available_size();
                let center = (0.5 * available_size).to_pos2();
                let board_size = available_size.min_elem();
                let ball_offset = board_size / 9.0;
                let ball_size = 0.8 * ball_offset;

                let painter = ui.painter();
                for (x, y, val) in self.game.iter() {
                    let center_idx = 4;
                    let cx = x - center_idx;
                    let cy = y - center_idx;
                    let x_offset = cx as f32 * Vec2::new(1.0, 0.0);
                    let y_offset = cy as f32 * Vec2::new(-0.8, 1.0);
                    let pos = center + ball_offset * (x_offset + y_offset);
                    match val {
                        Some(Color::Black) => {
                            painter.circle_filled(pos, 0.5 * ball_size, Color32::BLACK);
                        }
                        Some(Color::White) => {
                            painter.circle_filled(pos, 0.5 * ball_size, Color32::WHITE);
                        }
                        None => {
                            painter.circle_filled(pos, 0.5 * ball_size, Color32::GRAY);
                        }
                    }
                }
            });
    }
}
