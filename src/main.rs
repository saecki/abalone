use std::f32::consts::{FRAC_PI_4, FRAC_PI_6, PI, TAU};

use abalone::{Abalone, Color, Dir, SelectionError};
use eframe::NativeOptions;
use egui::{
    Align2, CentralPanel, Color32, FontFamily, FontId, Frame, Id, InputState, Key, Modifiers,
    Painter, Pos2, Rect, Rounding, Stroke, Ui, Vec2,
};

const BLACK_COLOR: Color32 = Color32::from_gray(0x02);
const WHITE_COLOR: Color32 = Color32::from_gray(0xD0);

const SELECTION_COLOR: Color32 = Color32::from_rgb(0x40, 0x60, 0xE0);
const SUCCESS_COLOR: Color32 = Color32::from_rgb(0x40, 0xF0, 0x60);
const WARN_COLOR: Color32 = Color32::from_rgb(0xF0, 0xE0, 0x40);
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
    Selection([abalone::Pos2; 2], Option<SelectionError>),
    Move([abalone::Pos2; 2], Result<abalone::Success, abalone::Error>),
}

struct Context {
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
            .frame(Frame::none().fill(Color32::from_gray(0x2B)))
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
                    center,
                    board_size,
                    ball_offset,
                    ball_radius,
                    line_thickness,
                    selection_radius,
                    board_angle,
                };

                ui.input_mut(|i| {
                    check_input(i, self, &ctx);
                });

                draw_game(ui, self, &ctx);
            });
    }
}

fn draw_game(ui: &mut Ui, app: &AbaloneApp, ctx: &Context) {
    let painter = ui.painter();

    let mut black_score = abalone::NUM_STARTING_BALLS;
    let mut white_score = abalone::NUM_STARTING_BALLS;
    for (_, _, c) in app.game.iter() {
        match c {
            Some(Color::Black) => white_score -= 1,
            Some(Color::White) => black_score -= 1,
            None => (),
        }
    }

    let board_rect = Rect::from_center_size(ctx.center, Vec2::splat(ctx.board_size));
    let padding = 0.2 * ctx.ball_offset;
    let font = FontId::new(ctx.ball_offset, FontFamily::Proportional);
    let black_score_pos = board_rect.right_top() + Vec2::new(-padding, padding);
    painter.text(
        black_score_pos,
        Align2::RIGHT_TOP,
        black_score.to_string(),
        font.clone(),
        BLACK_COLOR,
    );

    let white_score_pos = board_rect.left_top() + Vec2::new(padding, padding);
    painter.text(
        white_score_pos,
        Align2::LEFT_TOP,
        white_score.to_string(),
        font,
        WHITE_COLOR,
    );

    // balls
    for (x, y, val) in app.game.iter() {
        let pos = game_to_screen_pos(&ctx, (x, y).into());
        match val {
            Some(Color::Black) => {
                painter.circle_filled(pos, ctx.ball_radius, BLACK_COLOR);
            }
            Some(Color::White) => {
                painter.circle_filled(pos, ctx.ball_radius, WHITE_COLOR);
            }
            None => {
                let stroke = Stroke::new(ctx.line_thickness, Color32::from_gray(0x80));
                painter.circle_stroke(pos, ctx.selection_radius, stroke);
            }
        }
    }

    // highlight current state
    match &app.state {
        State::NoSelection => (),
        State::Selection(selection, error) => match error {
            &Some(SelectionError::WrongTurn(p)) => {
                let pos = game_to_screen_pos(&ctx, p);
                let error_stroke = Stroke::new(ctx.line_thickness, ERROR_COLOR);
                painter.circle_stroke(pos, ctx.selection_radius, error_stroke);

                let rect = Rect::from_center_size(pos, Vec2::splat(0.8 * ctx.ball_radius));
                painter.rect_filled(rect, Rounding::same(0.1 * ctx.ball_radius), ERROR_COLOR);
            }
            Some(SelectionError::InvalidSet) => {
                let [start, end] = *selection;
                highlight_one(painter, ctx, start, ERROR_COLOR);
                highlight_one(painter, ctx, end, ERROR_COLOR);
            }
            Some(SelectionError::MixedSet(mixed)) => {
                highlight_selection(painter, &ctx, *selection, SELECTION_COLOR);
                for &p in mixed.iter() {
                    highlight_one(painter, ctx, p, ERROR_COLOR);
                }
            }
            Some(SelectionError::NotABall(no_ball)) => {
                highlight_selection(painter, &ctx, *selection, SELECTION_COLOR);
                for &p in no_ball.iter() {
                    highlight_one(painter, ctx, p, ERROR_COLOR);
                }
            }
            Some(SelectionError::TooMany) => {
                highlight_selection(painter, &ctx, *selection, ERROR_COLOR);
            }
            Some(SelectionError::NoPossibleMove) => {
                highlight_selection(painter, &ctx, *selection, WARN_COLOR);
            }
            None => {
                highlight_selection(painter, &ctx, *selection, SELECTION_COLOR);
            }
        },
        State::Move(selection, res) => {
            highlight_selection(painter, &ctx, *selection, SELECTION_COLOR);
            match res {
                Err(abalone::Error::Selection(_)) => (),
                Err(abalone::Error::Move(e)) => match e {
                    &abalone::MoveError::PushedOff(p) => {
                        highlight_one(painter, ctx, p, ERROR_COLOR);
                    }
                    &abalone::MoveError::BlockedByOwn(p) => {
                        highlight_one(painter, ctx, p, ERROR_COLOR);
                    }
                    &abalone::MoveError::TooManyInferred { first, last } => {
                        highlight_selection(painter, &ctx, [first, last], ERROR_COLOR);
                    }
                    &abalone::MoveError::TooManyOpposing { first, last } => {
                        highlight_selection(painter, &ctx, [first, last], ERROR_COLOR);
                    }
                    abalone::MoveError::NotFree(not_free) => {
                        for &p in not_free.iter() {
                            highlight_one(painter, ctx, p, ERROR_COLOR);
                        }
                    }
                },
                Ok(success) => match success {
                    &abalone::Success::PushedOff { first, last } => {
                        let norm = (last - first).norm();
                        let selection = [first + norm, last];
                        highlight_selection(painter, &ctx, selection, SUCCESS_COLOR)
                    }
                    &abalone::Success::PushedAway { first, last } => {
                        let norm = (last - first).norm();
                        let selection = [first + norm, last + norm];
                        highlight_selection(painter, &ctx, selection, SUCCESS_COLOR)
                    }
                    &abalone::Success::Moved { dir, first, last } => {
                        let selection = [first + dir.vec(), last + dir.vec()];
                        highlight_selection(painter, &ctx, selection, SUCCESS_COLOR)
                    }
                },
            }
        }
    }

    match app.drag {
        Some((DragKind::Selection, start, end)) => {
            // center on selected ball
            let start = screen_to_game_pos(&ctx, start);
            let start = game_to_screen_pos(&ctx, start);

            let line_color = with_alpha(SELECTION_COLOR, 0x80);
            let stroke = Stroke::new(0.2 * ctx.ball_radius, line_color);
            painter.line_segment([start, end], stroke);
        }
        Some((DragKind::Direction, start, end)) => {
            let line_color = Color32::from_rgba_unmultiplied(0xF0, 0xA0, 0x40, 0x80);
            let stroke = Stroke::new(0.2 * ctx.ball_radius, line_color);
            painter.line_segment([start, end], stroke);

            // arrow tip
            let vec = end - start;
            if vec.length() > 0.5 * ctx.ball_offset {
                let tip_length = 0.25 * ctx.ball_offset;
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
        highlight_one(painter, ctx, p, color);
    }
}

fn highlight_one(painter: &Painter, ctx: &Context, pos: abalone::Pos2, color: Color32) {
    let pos = game_to_screen_pos(ctx, pos);
    let stroke = Stroke::new(ctx.line_thickness, color);
    painter.circle_stroke(pos, ctx.selection_radius, stroke);
}

fn check_input(i: &mut InputState, app: &mut AbaloneApp, ctx: &Context) {
    if i.consume_key(Modifiers::NONE, Key::Space) {
        app.board_flipped = !app.board_flipped;
    } else if i.consume_key(Modifiers::COMMAND, Key::Z) {
        app.game.undo_move();
    } else if i.consume_key(Modifiers::COMMAND, Key::Y) {
        app.game.redo_move();
    }

    if i.pointer.any_click() {
        if let Some(current) = i.pointer.interact_pos() {
            let pos = screen_to_game_pos(&ctx, current);
            if abalone::is_in_bounds(pos) {
                if i.pointer.secondary_released() {
                    // always discard selection if secondary click was used
                    let error = app.game.check_selection([pos; 2]).err();
                    app.state = State::Selection([pos; 2], error)
                } else {
                    match &app.state {
                        State::NoSelection => {
                            let error = app.game.check_selection([pos; 2]).err();
                            app.state = State::Selection([pos; 2], error)
                        }
                        &State::Selection([start, end], _) => {
                            let sel_vec = end - start;
                            if sel_vec == abalone::Vec2::ZERO {
                                if pos == start {
                                    app.state = State::NoSelection;
                                } else {
                                    let selection = [start, pos];
                                    let error = app.game.check_selection(selection).err();
                                    app.state = State::Selection(selection, error);
                                }
                            } else {
                                if pos == start {
                                    let selection = [start + sel_vec.norm(), end];
                                    let error = app.game.check_selection(selection).err();
                                    app.state = State::Selection(selection, error);
                                } else if pos == end {
                                    let selection = [start, end - sel_vec.norm()];
                                    let error = app.game.check_selection(selection).err();
                                    app.state = State::Selection(selection, error);
                                } else {
                                    let start_vec = pos - start;
                                    let end_vec = pos - end;
                                    if start_vec.is_multiple_of_unit_vec()
                                        && start_vec.is_parallel(sel_vec)
                                    {
                                        if start_vec.mag() < end_vec.mag() {
                                            let selection = [pos, end];
                                            let error = app.game.check_selection(selection).err();
                                            app.state = State::Selection(selection, error);
                                        } else {
                                            let selection = [start, pos];
                                            let error = app.game.check_selection(selection).err();
                                            app.state = State::Selection(selection, error);
                                        }
                                    }
                                }
                            }

                            // clear selection only if it's invalid set
                            if let State::Selection(_, Some(SelectionError::InvalidSet)) =
                                &app.state
                            {
                                app.state = State::NoSelection;
                            }
                        }
                        State::Move(_, _) => (),
                    }
                }
            } else {
                app.state = State::NoSelection;
            }
        }
    }
    if i.pointer.is_decidedly_dragging() {
        if let (Some(origin), Some(current)) = (i.pointer.press_origin(), i.pointer.interact_pos())
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
                        let error = app.game.check_selection([start, end]).err();
                        app.state = State::Selection([start, end], error);
                    } else {
                        app.state = State::NoSelection;
                    }
                }
                DragKind::Direction => {
                    match &app.state {
                        State::NoSelection => {
                            // use the start position as selection if there is none
                            if abalone::is_in_bounds(start) {
                                app.state = try_move(
                                    &app.game,
                                    [start; 2],
                                    [start, end],
                                    [origin, current],
                                );
                            }
                        }
                        State::Selection(selection, error) => {
                            if error.is_none() {
                                app.state = try_move(
                                    &app.game,
                                    *selection,
                                    [start, end],
                                    [origin, current],
                                );
                            }
                        }
                        State::Move(selection, _) => {
                            app.state =
                                try_move(&app.game, *selection, [start, end], [origin, current]);
                        }
                    }
                }
            }

            app.drag = Some((kind, origin, current));
        } else {
            // drag released
            match &app.state {
                State::NoSelection => (),
                State::Selection(_, error) => {
                    // clear invalid selection when drag is released
                    if error.is_some() {
                        app.state = State::NoSelection;
                    }
                }
                State::Move(selection, res) => {
                    app.state = match res {
                        Ok(success) => {
                            app.game.submit_move(*success);
                            State::NoSelection
                        }
                        Err(_) => State::Selection(*selection, None),
                    };
                }
            }
        }
    } else {
        app.drag = None;
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
