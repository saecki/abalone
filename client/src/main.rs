use std::f32::consts::{FRAC_PI_4, FRAC_PI_6, PI, TAU};

use abalone_core::{self as abalone, Abalone, Color, Dir, SelectionError};
use eframe::{CreationContext, NativeOptions};
use egui::{
    Align2, CentralPanel, Color32, FontFamily, FontId, Frame, Id, InputState, Key, Modifiers,
    Painter, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2,
};
use serde_derive::{Deserialize, Serialize};

const BLACK_COLOR: Color32 = Color32::from_gray(0x02);
const WHITE_COLOR: Color32 = Color32::from_gray(0xD0);
const ICON_COLOR: Color32 = Color32::from_gray(0xC0);
const ICON_DISABLED_COLOR: Color32 = Color32::from_gray(0x80);

const SELECTION_COLOR: Color32 = Color32::from_rgb(0x40, 0x60, 0xE0);
const SUCCESS_COLOR: Color32 = Color32::from_rgb(0x40, 0xF0, 0x60);
const WARN_COLOR: Color32 = Color32::from_rgb(0xF0, 0xE0, 0x40);
const ERROR_COLOR: Color32 = Color32::from_rgb(0xE0, 0x60, 0x40);

const ERROR_DISPLAY_TIME: f64 = 0.4;

fn main() {
    let native_options = NativeOptions {
        follow_system_theme: true,
        ..Default::default()
    };
    eframe::run_native(
        "abalone",
        native_options,
        Box::new(|cc| Box::new(AbaloneApp::new(cc))),
    )
    .expect("error running app");
}

#[derive(Serialize, Deserialize)]
struct AbaloneApp {
    game: Abalone,
    #[serde(skip)]
    drag: Option<(DragKind, Pos2, Pos2)>,
    #[serde(skip)]
    state: State,
    #[serde(skip)]
    input_errors: Vec<InputError>,
    board_flipped: bool,
}

impl Default for AbaloneApp {
    fn default() -> Self {
        Self {
            game: Abalone::new(),
            drag: None,
            state: State::NoSelection,
            input_errors: Vec::new(),
            board_flipped: false,
        }
    }
}

impl AbaloneApp {
    fn new(cc: &CreationContext) -> Self {
        if let Some(storage) = cc.storage {
            if let Some(app) = eframe::get_value(storage, eframe::APP_KEY) {
                return app;
            }
        }

        Self::default()
    }
}

enum InputError {
    WrongTurn {
        start_secs: f64,
        pos: abalone::Pos2,
    },
    InvalidSet {
        start_secs: f64,
        start: abalone::Pos2,
        end: abalone::Pos2,
    },
    CantExtendSelection {
        start_secs: f64,
        pos: abalone::Pos2,
    },
}

enum DragKind {
    Selection,
    Direction,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    NoSelection,
    Selection([abalone::Pos2; 2], Option<SelectionError>),
    Move([abalone::Pos2; 2], Result<abalone::Move, abalone::Error>),
}

struct Dimensions {
    screen_size: Vec2,
    center: Pos2,
    ball_offset: f32,
    ball_radius: f32,
    line_thickness: f32,
    selection_radius: f32,
    board_angle: f32,
}

impl eframe::App for AbaloneApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default()
            .frame(Frame::none().fill(Color32::from_gray(0x2B)))
            .show(ctx, |ui| {
                draw_game(ui, self);
            });
    }
}

fn draw_game(ui: &mut Ui, app: &mut AbaloneApp) {
    // TODO: fix animation snapping when changing direction while animation is still in progress.
    let board_angle = PI
        * ui.ctx()
            .animate_bool_with_time(Id::new("board_angle"), app.board_flipped, 0.3);

    let screen_size = ui.available_size();
    let center = (0.5 * screen_size).to_pos2() + Vec2::new(0.0, 0.05 * screen_size.min_elem());
    let board_size = 0.95 * screen_size.min_elem();
    let ball_offset = board_size / 9.0;
    let ball_radius = 0.4 * ball_offset;
    let line_thickness = 0.1 * ball_radius;
    let selection_radius = ball_radius - 0.5 * line_thickness;
    let dim = Dimensions {
        screen_size,
        center,
        ball_offset,
        ball_radius,
        line_thickness,
        selection_radius,
        board_angle,
    };

    ui.input_mut(|i| {
        check_input(i, app, &dim);
    });

    draw_board(ui, app, &dim);
}

fn draw_board(ui: &mut Ui, app: &mut AbaloneApp, dim: &Dimensions) {
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

    let used_screen_size = Vec2::splat(dim.screen_size.min_elem());
    let used_screen_pos = (0.5 * (dim.screen_size - used_screen_size)).to_pos2();
    let used_screen_rect = Rect::from_min_size(used_screen_pos, used_screen_size);

    let padding = 0.2 * dim.ball_offset;
    let score_font = FontId::new(dim.ball_offset, FontFamily::Proportional);
    let black_score_pos = used_screen_rect.right_top() + Vec2::new(-padding, padding);
    painter.text(
        black_score_pos,
        Align2::RIGHT_TOP,
        black_score.to_string(),
        score_font.clone(),
        BLACK_COLOR,
    );

    let white_score_pos = used_screen_rect.left_top() + Vec2::new(padding, padding);
    painter.text(
        white_score_pos,
        Align2::LEFT_TOP,
        white_score.to_string(),
        score_font,
        WHITE_COLOR,
    );

    let icon_font = FontId::new(0.4 * dim.ball_offset, FontFamily::Proportional);
    let undo_pos = used_screen_rect.center_top() + Vec2::new(-padding, padding);
    let color = if app.game.can_undo() {
        ICON_COLOR
    } else {
        ICON_DISABLED_COLOR
    };
    let rect = painter.text(
        undo_pos,
        Align2::RIGHT_TOP,
        "\u{2baa}".to_string(),
        icon_font.clone(),
        color,
    );
    if ui.interact(rect, Id::new("undo"), Sense::click()).clicked() {
        undo(app);
    }

    let redo_pos = used_screen_rect.center_top() + Vec2::new(padding, padding);
    let color = if app.game.can_redo() {
        ICON_COLOR
    } else {
        ICON_DISABLED_COLOR
    };
    let rect = painter.text(
        redo_pos,
        Align2::LEFT_TOP,
        "\u{2bab}".to_string(),
        icon_font,
        color,
    );
    if ui.interact(rect, Id::new("redo"), Sense::click()).clicked() {
        redo(app);
    }

    // balls
    for (x, y, val) in app.game.iter() {
        let pos = game_to_screen_pos(dim, (x, y).into());
        match val {
            Some(Color::Black) => {
                painter.circle_filled(pos, dim.ball_radius, BLACK_COLOR);
            }
            Some(Color::White) => {
                painter.circle_filled(pos, dim.ball_radius, WHITE_COLOR);
            }
            None => {
                let stroke = Stroke::new(dim.line_thickness, Color32::from_gray(0x80));
                painter.circle_stroke(pos, dim.selection_radius, stroke);
            }
        }
    }

    // highlight current state
    match &app.state {
        State::NoSelection => (),
        State::Selection(selection, error) => match error {
            &Some(SelectionError::WrongTurn(p)) => {
                highlight_one_square(painter, dim, p, ERROR_COLOR);

                let [start, end] = *selection;
                if start != end {
                    highlight_one(painter, dim, end, ERROR_COLOR);
                }
            }
            Some(SelectionError::InvalidSet) => {
                let [start, end] = *selection;
                highlight_one(painter, dim, start, ERROR_COLOR);
                highlight_one(painter, dim, end, ERROR_COLOR);
            }
            Some(SelectionError::MixedSet(mixed)) => {
                highlight_selection(painter, dim, *selection, SELECTION_COLOR);
                for &p in mixed.iter() {
                    highlight_one(painter, dim, p, ERROR_COLOR);
                }
            }
            Some(SelectionError::NotABall(no_ball)) => {
                highlight_selection(painter, dim, *selection, SELECTION_COLOR);
                for &p in no_ball.iter() {
                    highlight_one(painter, dim, p, ERROR_COLOR);
                }
            }
            Some(SelectionError::TooMany) => {
                highlight_selection(painter, dim, *selection, ERROR_COLOR);
            }
            Some(SelectionError::NoPossibleMove) => {
                highlight_selection(painter, dim, *selection, WARN_COLOR);
            }
            None => {
                highlight_selection(painter, dim, *selection, SELECTION_COLOR);
            }
        },
        State::Move(selection, res) => {
            highlight_selection(painter, dim, *selection, SELECTION_COLOR);
            match res {
                Err(abalone::Error::Selection(_)) => (),
                Err(abalone::Error::Move(e)) => match e {
                    abalone::MoveError::PushedOff(pushed_off) => {
                        for &p in pushed_off.iter() {
                            highlight_one(painter, dim, p, ERROR_COLOR);
                        }
                    }
                    &abalone::MoveError::BlockedByOwn(p) => {
                        highlight_one(painter, dim, p, ERROR_COLOR);
                    }
                    &abalone::MoveError::TooManyInferred { first, last } => {
                        highlight_selection(painter, dim, [first, last], ERROR_COLOR);
                    }
                    &abalone::MoveError::TooManyOpposing { first, last } => {
                        highlight_selection(painter, dim, [first, last], ERROR_COLOR);
                    }
                    abalone::MoveError::NotFree(not_free) => {
                        for &p in not_free.iter() {
                            highlight_one(painter, dim, p, ERROR_COLOR);
                        }
                    }
                },
                Ok(mov) => match *mov {
                    abalone::Move::PushedOff { first, last } => {
                        let norm = (last - first).norm();
                        let selection = [first + norm, last];
                        highlight_selection(painter, dim, selection, SUCCESS_COLOR)
                    }
                    abalone::Move::PushedAway { first, last } => {
                        let norm = (last - first).norm();
                        let selection = [first + norm, last + norm];
                        highlight_selection(painter, dim, selection, SUCCESS_COLOR)
                    }
                    abalone::Move::Moved { dir, first, last } => {
                        let selection = [first + dir.vec(), last + dir.vec()];
                        highlight_selection(painter, dim, selection, SUCCESS_COLOR)
                    }
                },
            }
        }
    }

    for e in app.input_errors.iter() {
        // request repaint so the input errors will be cleared
        ui.ctx().request_repaint();

        match *e {
            InputError::WrongTurn { pos, .. } => {
                highlight_one_square(painter, dim, pos, ERROR_COLOR);
            }
            InputError::InvalidSet { start, end, .. } => {
                highlight_one(painter, dim, start, ERROR_COLOR);
                highlight_one(painter, dim, end, ERROR_COLOR);
            }
            InputError::CantExtendSelection { pos, .. } => {
                highlight_one(painter, dim, pos, ERROR_COLOR);
            }
        };
    }

    match app.drag {
        Some((DragKind::Selection, start, end)) => {
            // center on selected ball
            let start = screen_to_game_pos(dim, start);
            let start = game_to_screen_pos(dim, start);

            let line_color = with_alpha(SELECTION_COLOR, 0x80);
            let stroke = Stroke::new(0.2 * dim.ball_radius, line_color);
            painter.line_segment([start, end], stroke);
        }
        Some((DragKind::Direction, start, end)) => {
            let line_color = Color32::from_rgba_unmultiplied(0xF0, 0xA0, 0x40, 0x80);
            let stroke = Stroke::new(0.2 * dim.ball_radius, line_color);
            painter.line_segment([start, end], stroke);

            // arrow tip
            let vec = end - start;
            if vec.length() > 0.5 * dim.ball_offset {
                let tip_length = 0.25 * dim.ball_offset;
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
    dim: &Dimensions,
    selection: [abalone::Pos2; 2],
    color: Color32,
) {
    let [start, end] = selection;
    let vec = end - start;
    let norm = vec.norm();
    let mag = vec.mag();
    for i in 0..=mag {
        let p = start + norm * i;
        highlight_one(painter, dim, p, color);
    }
}

fn highlight_one_square(painter: &Painter, dim: &Dimensions, pos: abalone::Pos2, color: Color32) {
    let pos = game_to_screen_pos(dim, pos);
    let stroke = Stroke::new(dim.line_thickness, color);
    painter.circle_stroke(pos, dim.selection_radius, stroke);
    let rect = Rect::from_center_size(pos, Vec2::splat(0.8 * dim.ball_radius));
    painter.rect_filled(rect, Rounding::same(0.1 * dim.ball_radius), color);
}

fn highlight_one(painter: &Painter, dim: &Dimensions, pos: abalone::Pos2, color: Color32) {
    let pos = game_to_screen_pos(dim, pos);
    let stroke = Stroke::new(dim.line_thickness, color);
    painter.circle_stroke(pos, dim.selection_radius, stroke);
}

fn check_input(i: &mut InputState, app: &mut AbaloneApp, dim: &Dimensions) {
    if i.consume_key(Modifiers::NONE, Key::Space) {
        app.board_flipped = !app.board_flipped;
    } else if i.consume_key(Modifiers::COMMAND, Key::Z) {
        undo(app);
    } else if i.consume_key(Modifiers::COMMAND, Key::Y) {
        redo(app);
    } else if i.consume_key(Modifiers::NONE, Key::Escape) {
        app.state = State::NoSelection;
    }

    if i.pointer.any_click() {
        if let Some(current) = i.pointer.interact_pos() {
            let pos = screen_to_game_pos(dim, current);
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
                            } else if pos == start {
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
                                } else {
                                    app.input_errors.push(InputError::CantExtendSelection {
                                        start_secs: i.time,
                                        pos,
                                    });
                                }
                            }
                        }
                        State::Move(_, _) => (),
                    }
                }

                // clear selection only if it's invalid set
                if let State::Selection(selection, error) = &app.state {
                    match error {
                        Some(SelectionError::WrongTurn(p)) => {
                            app.input_errors.push(InputError::WrongTurn {
                                start_secs: i.time,
                                pos: *p,
                            });
                            app.state = State::NoSelection;
                        }
                        Some(SelectionError::InvalidSet) => {
                            let [start, end] = *selection;
                            app.input_errors.push(InputError::InvalidSet {
                                start_secs: i.time,
                                start,
                                end,
                            });
                            app.state = State::NoSelection;
                        }
                        _ => (),
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
            let start = screen_to_game_pos(dim, origin);

            match kind {
                DragKind::Selection => {
                    let end = screen_to_game_pos(dim, current);
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
                                app.state = try_move(&app.game, dim, [start; 2], [origin, current]);
                            }
                        }
                        State::Selection(selection, error) => {
                            if error.is_none() {
                                app.state = try_move(&app.game, dim, *selection, [origin, current]);
                            }
                        }
                        State::Move(selection, _) => {
                            app.state = try_move(&app.game, dim, *selection, [origin, current]);
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
                        Ok(mov) => {
                            app.game.submit_move(*mov);
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

    app.input_errors.retain(|e| {
        let start = match e {
            InputError::WrongTurn { start_secs, .. }
            | InputError::InvalidSet { start_secs, .. }
            | InputError::CantExtendSelection { start_secs, .. } => start_secs,
        };
        start + ERROR_DISPLAY_TIME > i.time
    });
}

fn undo(app: &mut AbaloneApp) {
    app.state = State::NoSelection;
    app.game.undo_move();
}

fn redo(app: &mut AbaloneApp) {
    app.state = State::NoSelection;
    app.game.redo_move();
}

fn try_move(
    game: &Abalone,
    dim: &Dimensions,
    selection: [abalone::Pos2; 2],
    [origin, current]: [Pos2; 2],
) -> State {
    let drag_vec = current - origin;
    if drag_vec.length() < 0.5 * dim.ball_offset {
        let error = game.check_selection(selection).err();
        return State::Selection(selection, error);
    }

    let angle = (6.0 * ((drag_vec.angle() - dim.board_angle + TAU) % TAU) / TAU).round();
    let idx = (angle as u8) % 6;
    let dir = match idx {
        0 => Dir::PosX,
        1 => Dir::PosZ,
        2 => Dir::PosY,
        3 => Dir::NegX,
        4 => Dir::NegZ,
        5 => Dir::NegY,
        _ => unreachable!(),
    };

    let res = game.check_move(selection, dir);
    State::Move(selection, res)
}

fn game_to_screen_pos(dim: &Dimensions, pos: abalone::Pos2) -> Pos2 {
    let center_idx = 4;
    let cx = pos.x - center_idx;
    let cy = pos.y - center_idx;
    let unit_x = rot_vec2(dim.board_angle, Vec2::new(1.0, 0.0));
    let unit_y = rot_vec2(dim.board_angle + FRAC_PI_6, Vec2::new(0.0, 1.0));
    dim.center + dim.ball_offset * (cx as f32 * unit_x + cy as f32 * unit_y)
}

fn screen_to_game_pos(dim: &Dimensions, pos: Pos2) -> abalone::Pos2 {
    let center_dist = pos - dim.center;
    if center_dist == Vec2::ZERO {
        return abalone::Pos2::ZERO;
    }

    let unit_x = rot_vec2(dim.board_angle, Vec2::new(1.0, 0.0));
    let unit_y = rot_vec2(dim.board_angle + FRAC_PI_6, Vec2::new(0.0, 1.0));
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

    let cx = (a / dim.ball_offset).round() as i8;
    let cy = (b / dim.ball_offset).round() as i8;

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
