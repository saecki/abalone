use std::f32::consts::{FRAC_PI_4, FRAC_PI_6, PI, TAU};

use abalone::{Abalone, Color, Dir};
use eframe::NativeOptions;
use egui::{CentralPanel, Color32, Frame, Id, Key, Modifiers, Painter, Pos2, Stroke, Vec2};

const SELECTION_COLOR: Color32 = Color32::from_rgb(0x40, 0x60, 0xE0);
const WARN_COLOR: Color32 = Color32::from_rgb(0xE0, 0xE0, 0x40);
const ERROR_COLOR: Color32 = Color32::from_rgb(0xE0, 0x60, 0x40);

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
    drag: Option<(DragKind, Pos2, Pos2)>,
    state: State,
    board_flipped: bool,
}

impl AbaloneApp {
    fn new() -> Self {
        Self {
            game: Abalone::new(),
            drag: None,
            state: State::NoSelection,
            board_flipped: false,
        }
    }
}

enum DragKind {
    Selection,
    Direction,
}

#[derive(Debug)]
enum State {
    NoSelection,
    Selection([abalone::Pos2; 2], Option<abalone::SelectionError>),
    Move([abalone::Pos2; 2], Result<abalone::Success, abalone::Error>),
}

struct Context {
    screen_size: Vec2,
    center: Pos2,
    board_size: f32,
    ball_offset: f32,
    ball_radius: f32,
    line_thickness: f32,
    selection_radius: f32,
    board_angle: f32,
}

impl eframe::App for AbaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default()
            .frame(Frame::none().fill(Color32::from_gray(0x30)))
            .show(ctx, |ui| {
                // TODO: fix animation snapping when changing direction while animation is still in progress.
                let board_angle = PI
                    * ctx.animate_bool_with_time(Id::new("board_angle"), self.board_flipped, 0.3);

                let screen_size = ui.available_size();
                let center = (0.5 * screen_size).to_pos2();
                let board_size = screen_size.min_elem();
                let ball_offset = board_size / 9.0;
                let ball_radius = 0.4 * ball_offset;
                let line_thickness = 0.1 * ball_radius;
                let selection_radius = ball_radius - 0.5 * line_thickness;
                let ctx = Context {
                    screen_size,
                    center,
                    board_size,
                    ball_offset,
                    ball_radius,
                    line_thickness,
                    selection_radius,
                    board_angle,
                };

                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::Space) {
                        self.board_flipped = !self.board_flipped;
                    }

                    if i.pointer.secondary_pressed() {
                        if let Some(current) = i.pointer.interact_pos() {
                            let pos = screen_to_game_pos(&ctx, current);
                            if abalone::is_in_bounds(pos) {
                                let error = self.game.check_selection([pos; 2]).err();
                                self.state = State::Selection([pos; 2], error)
                            } else {
                                self.state = State::NoSelection;
                            }
                        }
                    }
                    if i.pointer.is_decidedly_dragging() {
                        if let (Some(origin), Some(current)) =
                            (i.pointer.press_origin(), i.pointer.interact_pos())
                        {
                            let kind = if i.pointer.primary_down() {
                                DragKind::Direction
                            } else {
                                DragKind::Selection
                            };
                            let start = screen_to_game_pos(&ctx, origin);
                            let end = screen_to_game_pos(&ctx, current);

                            match kind {
                                DragKind::Selection => {
                                    if abalone::is_in_bounds(start) && abalone::is_in_bounds(end) {
                                        let error = self.game.check_selection([start, end]).err();
                                        self.state = State::Selection([start, end], error);
                                    } else {
                                        self.state = State::NoSelection;
                                    }
                                }
                                DragKind::Direction => {
                                    match &self.state {
                                        // TODO: consider allowing moves of single balls, without
                                        // any selection
                                        State::NoSelection => (),
                                        State::Selection(selection, error) => {
                                            if error.is_none() {
                                                self.state = try_move(
                                                    &self.game,
                                                    *selection,
                                                    [start, end],
                                                    [origin, current],
                                                );
                                            }
                                        }
                                        State::Move(selection, _) => {
                                            self.state = try_move(
                                                &self.game,
                                                *selection,
                                                [start, end],
                                                [origin, current],
                                            );
                                        }
                                    }
                                }
                            }

                            self.drag = Some((kind, origin, current));
                        }
                    } else {
                        self.drag = None;
                    }
                });

                let painter = ui.painter();

                // balls
                for (x, y, val) in self.game.iter() {
                    let pos = game_to_screen_pos(&ctx, (x, y).into());
                    match val {
                        Some(Color::Black) => {
                            painter.circle_filled(pos, ball_radius, Color32::BLACK);
                        }
                        Some(Color::White) => {
                            painter.circle_filled(pos, ball_radius, Color32::WHITE);
                        }
                        None => {
                            let stroke = Stroke::new(line_thickness, Color32::from_gray(0x80));
                            painter.circle_stroke(pos, selection_radius, stroke);
                        }
                    }
                }

                // highlight current state
                let error_stroke = Stroke::new(line_thickness, ERROR_COLOR);
                match &self.state {
                    State::NoSelection => (),
                    State::Selection(selection, error) => match error {
                        Some(abalone::SelectionError::InvalidSet) => {
                            let [start, end] = *selection;
                            let pos = game_to_screen_pos(&ctx, start);
                            painter.circle_stroke(pos, selection_radius, error_stroke);
                            let pos = game_to_screen_pos(&ctx, end);
                            painter.circle_stroke(pos, selection_radius, error_stroke);
                        }
                        Some(abalone::SelectionError::MixedSet(mixed)) => {
                            for &p in mixed.iter() {
                                let pos = game_to_screen_pos(&ctx, p);
                                painter.circle_stroke(pos, selection_radius, error_stroke);
                            }
                        }
                        Some(abalone::SelectionError::NotABall(p)) => {
                            let pos = game_to_screen_pos(&ctx, *p);
                            painter.circle_stroke(pos, selection_radius, error_stroke);
                        }
                        Some(abalone::SelectionError::TooMany) => {
                            highlight_selection(painter, &ctx, *selection, ERROR_COLOR);
                        }
                        Some(abalone::SelectionError::NoPossibleMove) => {
                            highlight_selection(painter, &ctx, *selection, WARN_COLOR);
                        }
                        None => {
                            highlight_selection(painter, &ctx, *selection, SELECTION_COLOR);
                        }
                    },
                    State::Move(selection, res) => {
                        match res {
                            Err(abalone::Error::Selection(_)) => (),
                            Err(abalone::Error::Move(e)) => match e {
                                abalone::MoveError::PushedOff(_) => todo!(),
                                abalone::MoveError::BlockedByOwn(_) => todo!(),
                                abalone::MoveError::TooManyInferred { first, last } => todo!(),
                                abalone::MoveError::TooManyOpposing { first, last } => todo!(),
                                abalone::MoveError::NotFree(_) => todo!(),
                            },
                            Ok(success) => {
                                // TODO: display move
                            }
                        }
                    }
                }

                match self.drag {
                    Some((DragKind::Selection, start, end)) => {
                        // center on selected ball
                        let start = screen_to_game_pos(&ctx, start);
                        let start = game_to_screen_pos(&ctx, start);

                        let line_color = with_alpha(SELECTION_COLOR, 0x80);
                        let stroke = Stroke::new(0.2 * ball_radius, line_color);
                        painter.line_segment([start, end], stroke);
                    }
                    Some((DragKind::Direction, start, end)) => {
                        let line_color = Color32::from_rgba_unmultiplied(0xF0, 0xA0, 0x40, 0x80);
                        let stroke = Stroke::new(0.2 * ball_radius, line_color);
                        painter.line_segment([start, end], stroke);

                        // arrow tip
                        let vec = end - start;
                        if vec.length() > 0.5 * ball_offset {
                            let tip_length = 0.25 * ball_offset;
                            let arrow_angle = vec.angle();
                            let left_tip_angle = arrow_angle - FRAC_PI_4;
                            let right_tip_angle = arrow_angle + FRAC_PI_4;
                            let tip_left = end
                                - Vec2::new(
                                    left_tip_angle.cos() * tip_length,
                                    left_tip_angle.sin() * tip_length,
                                );
                            let tip_right = end
                                - Vec2::new(
                                    right_tip_angle.cos() * tip_length,
                                    right_tip_angle.sin() * tip_length,
                                );
                            painter.line_segment([end, tip_left], stroke);
                            painter.line_segment([end, tip_right], stroke);
                        }
                    }
                    None => (),
                }
            });
    }
}

fn highlight_selection(
    painter: &Painter,
    ctx: &Context,
    selection: [abalone::Pos2; 2],
    color: Color32,
) {
    let [start, end] = selection;
    let vec = end - start;
    let norm = vec.norm();
    let mag = vec.mag();
    for i in 0..=mag {
        let p = start + norm * i;
        let pos = game_to_screen_pos(ctx, p);
        let stroke = Stroke::new(ctx.line_thickness, color);
        painter.circle_stroke(pos, ctx.selection_radius, stroke);
    }
}

fn try_move(
    game: &Abalone,
    selection: [abalone::Pos2; 2],
    [start, end]: [abalone::Pos2; 2],
    [origin, current]: [Pos2; 2],
) -> State {
    let dir_vec = end - start;
    if dir_vec == abalone::Vec2::ZERO {
        let error = game.check_selection(selection).err();
        return State::Selection(selection, error);
    }

    let dir_norm = dir_vec.norm();
    if let Some(dir) = dir_norm.unit_vec() {
        let res = game.check_move(selection, dir);
        return State::Move(selection, res);
    }

    let drag_vec = current - origin;
    let angle = (6.0 * drag_vec.angle() / TAU).round();
    let idx = (angle as u8) % 6;
    let dir = match idx {
        0 => Dir::PosX,
        1 => Dir::NegY,
        2 => Dir::NegZ,
        3 => Dir::NegX,
        4 => Dir::PosY,
        5 => Dir::PosZ,
        _ => unreachable!(),
    };

    let res = game.check_move(selection, dir);
    State::Move(selection, res)
}

fn game_to_screen_pos(ctx: &Context, pos: abalone::Pos2) -> Pos2 {
    let center_idx = 4;
    let cx = pos.x - center_idx;
    let cy = pos.y - center_idx;
    let unit_x = rot_vec2(ctx.board_angle, Vec2::new(1.0, 0.0));
    let unit_y = rot_vec2(ctx.board_angle + FRAC_PI_6, Vec2::new(0.0, 1.0));
    ctx.center + ctx.ball_offset * (cx as f32 * unit_x + cy as f32 * unit_y)
}

fn screen_to_game_pos(ctx: &Context, pos: Pos2) -> abalone::Pos2 {
    let center_dist = pos - ctx.center;
    if center_dist == Vec2::ZERO {
        return abalone::Pos2::ZERO;
    }

    let unit_x = rot_vec2(ctx.board_angle, Vec2::new(1.0, 0.0));
    let unit_y = rot_vec2(ctx.board_angle + FRAC_PI_6, Vec2::new(0.0, 1.0));
    let ux = unit_x.x;
    let uy = unit_x.y;
    let vx = unit_y.x;
    let vy = unit_y.y;
    let c = center_dist.x;
    let d = center_dist.y;

    // # Find game pos by solving equation system
    // I :  ux * a + vx * b = c
    // II:  uy * a + vy * b = d
    //
    // # I * uy - II * ux
    // uy * (ux * a + vx * b) - ux * (uy * a + vy * b) = uy * c - ux * d
    //   ux*uy * a + vx*uy * b - ux*uy * a + ux*vy * b = uy * c - ux * d
    //               vx*uy * b             - ux*vy * b = uy * c - ux * d
    //                             (vx*uy - ux*vy) * b = uy * c - ux * d
    //                                               b = (uy * c - ux * d) / (vx*uy - ux*vy)
    //
    // # Replace b in I
    // ux * a + vx = c
    // ux * a          = (c - b * vx)
    //      a          = (c - b * vx) / ux
    let b = (uy * c - ux * d) / (vx * uy - ux * vy);
    let a = (c - b * vx) / ux;

    let cx = (a / ctx.ball_offset).round() as i8;
    let cy = (b / ctx.ball_offset).round() as i8;

    let center_idx = 4;
    abalone::Pos2 {
        x: cx + center_idx,
        y: cy + center_idx,
    }
}

fn rot_vec2(angle: f32, vec: Vec2) -> Vec2 {
    Vec2::new(
        vec.x * angle.cos() + vec.y * -angle.sin(),
        vec.x * angle.sin() + vec.y * angle.cos(),
    )
}

fn with_alpha(color: Color32, a: u8) -> Color32 {
    let [r, g, b, _] = color.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}
