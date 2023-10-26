use std::f32::consts::FRAC_PI_6;

use abalone::{Abalone, Color};
use egui::{CentralPanel, Frame, Shape};

fn main() {
    let game = Abalone::new();

    println!("{:#?}", game);
}

struct AbaloneApp {
    game: Abalone,
}

impl eframe::App for AbaloneApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let available_size = ui.available_size();
                let center = 0.5 * available_size;
                let board_size = center.min_elem();

                let painter = ui.painter();
                for (x, y, val) in self.game.iter() {
                    match val {
                        Some(Color::Black) => {
                            painter.circle_filled(center, radius, fill_color)
                        }
                        Some(Color::White) => {

                        }
                        None => {

                        }
                    }
                }
            });
    }
}
