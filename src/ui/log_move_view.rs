//! The question asked when the logs folder changes, and the window the move
//! then runs in: two folders and a sheet of paper flying from one to the
//! other, the way a file copy has looked since Windows 95.

use std::f32::consts::PI;

use egui::{pos2, vec2, Color32, Context, Pos2, Rect, Sense, Shape, Stroke, Ui};

use crate::features::log_move::{size, Job, Plan};
use crate::i18n::{tr, tr1, tr2};

/// Where a change of the logs folder has got to.
pub enum LogMove {
    /// Files were found in the old folder, and the user is asked whether to
    /// take them along.
    Asking(Plan),
    /// They are being moved, or have been.
    Moving(Job),
}

/// How long one sheet takes to cross, in seconds.
const FLIGHT: f64 = 0.9;

/// Draws whichever of the two `state` is at, in `ctx`'s window, and clears
/// `state` once the user is done with it.
pub fn show(ctx: &Context, state: &mut Option<LogMove>) {
    let Some(current) = state.as_mut() else {
        return;
    };
    let mut open = true;
    let mut next = None;
    let mut done = false;
    use crate::ui::dialog::{actions, button, Role};
    match current {
        LogMove::Asking(plan) => {
            crate::ui::dialog::show(ctx, "nit-log-move", tr("Move the logs?"), &mut open, |ui| {
                ui.set_max_width(440.0);
                let count = plan.files.len();
                ui.label(match count {
                    1 => tr1(
                        "There is 1 log ({}) in the old folder.",
                        &size(plan.bytes()),
                    ),
                    n => tr2(
                        "There are {} logs ({}) in the old folder.",
                        &n.to_string(),
                        &size(plan.bytes()),
                    ),
                });
                folder_line(ui, tr("From:"), &plan.from.display().to_string());
                folder_line(ui, tr("To:"), &plan.to.display().to_string());
                ui.weak(tr(
                    "New transcripts already go to the new folder. Files that are not the app's own stay where they are.",
                ));
                actions(ui, |ui| {
                    if button(ui, tr("Move them"), Role::Suggested).clicked() {
                        next = Some(true);
                    }
                    if button(ui, tr("Leave them there"), Role::Plain).clicked() {
                        next = Some(false);
                    }
                });
            });
            if !open || next == Some(false) {
                done = true;
            } else if next == Some(true) {
                let plan = std::mem::take(plan);
                let waker = ctx.clone();
                let viewport = ctx.viewport_id();
                *current =
                    LogMove::Moving(Job::start(plan, move || waker.request_repaint_of(viewport)));
            }
        }
        LogMove::Moving(job) => {
            job.poll();
            let p = job.progress.clone();
            let total = job.plan.files.len();
            let bytes = job.plan.bytes();
            let title = if p.finished {
                tr("Logs moved")
            } else {
                tr("Moving logs")
            };
            crate::ui::dialog::show(ctx, "nit-log-move", title, &mut open, |ui| {
                ui.set_width(420.0);
                flight(ui, !p.finished);
                if p.finished {
                    let moved = p.done - p.failed.len();
                    ui.label(if p.cancelled {
                        tr2(
                            "Stopped: {} of {} moved.",
                            &moved.to_string(),
                            &total.to_string(),
                        )
                    } else {
                        tr1("{} moved.", &moved.to_string())
                    });
                    for (name, why) in p.failed.iter().take(5) {
                        ui.colored_label(ui.visuals().error_fg_color, format!("{name}: {why}"));
                    }
                    if p.failed.len() > 5 {
                        ui.weak(tr1("and {} more.", &(p.failed.len() - 5).to_string()));
                    }
                } else {
                    ui.label(if p.current.is_empty() {
                        tr("Starting...").to_string()
                    } else {
                        p.current.clone()
                    });
                }
                let fraction = if bytes == 0 {
                    p.done as f32 / total.max(1) as f32
                } else {
                    p.bytes_done as f32 / bytes as f32
                };
                ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).desired_width(420.0));
                ui.weak(format!(
                    "{} · {}",
                    tr2("{} of {} files", &p.done.to_string(), &total.to_string()),
                    tr2("{} of {}", &size(p.bytes_done.min(bytes)), &size(bytes)),
                ));
                actions(ui, |ui| {
                    if p.finished {
                        if button(ui, tr("Close"), Role::Suggested).clicked() {
                            done = true;
                        }
                    } else if button(ui, tr("Stop"), Role::Plain).clicked() {
                        job.cancel();
                    }
                });
            });
            // Closing the window while it works stops it after the current
            // file, as Stop does - never part way through one.
            if !open {
                if p.finished {
                    done = true;
                } else {
                    job.cancel();
                }
            }
            if !p.finished {
                // The sheet is moving even when no file has finished.
                ctx.request_repaint();
            }
        }
    }
    if done {
        *state = None;
    }
}

fn folder_line(ui: &mut Ui, label: &str, path: &str) {
    ui.horizontal(|ui| {
        ui.weak(label);
        ui.monospace(path);
    });
}

/// The two folders and, while `moving`, a sheet arcing between them.
fn flight(ui: &mut Ui, moving: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 84.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let ink = ui.visuals().strong_text_color();
    let from = pos2(rect.left() + 56.0, rect.bottom() - 22.0);
    let to = pos2(rect.right() - 56.0, rect.bottom() - 22.0);
    painter.extend(folder(from, ink, true));
    painter.extend(folder(to, ink, false));
    if moving {
        let t = (ui.input(|i| i.time) / FLIGHT).fract() as f32;
        // Up over the gap and down into the other folder, turning as it goes.
        let x = from.x + (to.x - from.x) * t;
        let y = from.y - 18.0 - (PI * t).sin() * 34.0;
        let turn = (t - 0.5) * 0.9;
        painter.extend(sheet(pos2(x, y), turn, ink));
    }
}

/// A manila folder standing at `at`, open at the top when it is the one the
/// sheets leave.
fn folder(at: Pos2, ink: Color32, open: bool) -> Vec<Shape> {
    let body = Color32::from_rgb(0xe8, 0xc2, 0x5a);
    let back = Color32::from_rgb(0xc9, 0x9f, 0x3a);
    let edge = Stroke::new(1.0_f32, ink.gamma_multiply(0.6));
    let w = 46.0;
    let h = 32.0;
    let base = Rect::from_center_size(at, vec2(w, h));
    let tab = Rect::from_min_size(base.min - vec2(0.0, 6.0), vec2(18.0, 7.0));
    let mut shapes = vec![
        Shape::rect_filled(tab, 2.0, back),
        Shape::rect_filled(base, 2.0, back),
        Shape::rect_stroke(base, 2.0, edge),
    ];
    // The front flap: tipped forward on the folder being emptied.
    let lean = if open { 6.0 } else { 0.0 };
    let front = vec![
        pos2(base.left() - lean, base.top() + 6.0),
        pos2(base.right() + lean, base.top() + 6.0),
        pos2(base.right(), base.bottom()),
        pos2(base.left(), base.bottom()),
    ];
    shapes.push(Shape::convex_polygon(front, body, edge));
    shapes
}

/// One sheet of paper centred on `at`, turned by `turn` radians, with a few
/// lines of text on it.
fn sheet(at: Pos2, turn: f32, ink: Color32) -> Vec<Shape> {
    let (s, c) = turn.sin_cos();
    let point = |x: f32, y: f32| pos2(at.x + x * c - y * s, at.y + x * s + y * c);
    let (w, h) = (9.0, 12.0);
    let outline = vec![point(-w, -h), point(w, -h), point(w, h), point(-w, h)];
    let mut shapes = vec![Shape::convex_polygon(
        outline,
        Color32::WHITE,
        Stroke::new(1.0_f32, ink.gamma_multiply(0.7)),
    )];
    for row in 0..4 {
        let y = -h + 5.0 + row as f32 * 5.0;
        shapes.push(Shape::line_segment(
            [
                point(-w + 3.0, y),
                point(w - 3.0 - (row % 2) as f32 * 4.0, y),
            ],
            Stroke::new(1.0_f32, ink.gamma_multiply(0.35)),
        ));
    }
    shapes
}
