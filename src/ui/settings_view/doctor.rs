//! The Doctor page: the organisation's environment checks, each with a status
//! light, opening to say more and to offer its fix.
//!
//! The checks run when the page is first opened and when Re-run is pressed,
//! never on their own: the engine in [`crate::features::doctor`] runs them on
//! a thread and wakes the window as each one finishes.

use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use egui::{Color32, Rect, RichText, Rounding, Ui};

use super::{section, Ctx, Item, Section};
use crate::features::doctor::run::{self, Context};
use crate::features::doctor::spec::{self, Check, Status};
use crate::features::doctor::{is_issue, Doctor};
use crate::i18n::{tr, tr1};
use crate::ui::file_dialog;
use crate::ui::panels::UiRequest;
use crate::ui::prefs::{self, Card, Row};
use crate::ui::tip::Tip;

/// What the page keeps between frames besides the results themselves.
#[derive(Default)]
pub struct DoctorPageState {
    /// The checks shown open.
    expanded: HashSet<String>,
    /// The check whose fix is asking to be confirmed.
    confirming: Option<String>,
    /// Why the last fix could not be opened, and for which check.
    fix_error: Option<(String, String)>,
    /// The report was copied, until the next run.
    copied: bool,
    /// The file dialog choosing the checks file, while it is open.
    picking: Option<Receiver<Option<String>>>,
}

impl DoctorPageState {
    /// The page with `ids` open, for a snapshot.
    #[cfg(test)]
    pub fn opened(ids: &[&str]) -> Self {
        DoctorPageState {
            expanded: ids.iter().map(|id| id.to_string()).collect(),
            ..DoctorPageState::default()
        }
    }
}

/// Something pressed on the page, carried out once it is drawn: the drawing
/// borrows the results it would change.
enum Action {
    Toggle(String),
    RunOne(String),
    AskFix(String),
    CancelFix,
    Fix(String),
    Verify(String),
}

pub(super) fn sections() -> Vec<Section> {
    vec![
        super::drawn(checks),
        section(
            "Checks file",
            vec![Item::rows("doctor_path", "Path", path_row).keys(&[
                "doctor", "health", "saúde", "checks", "verificações", "xml", "ambiente",
                "environment",
            ])],
        )
        .footer("The checks are written by your organisation: an XML file naming Python, PowerShell, shell or ObjectScript scripts, each answering with a status. See docs/doctor.md."),
    ]
}

/// What the checks are run against: the file's own settings, then the
/// default session's instance, then the first one installed.
fn context(c: &Ctx<'_>, spec: &spec::Spec) -> Context {
    let settings = &*c.settings;
    let default = settings
        .profiles
        .iter()
        .find(|p| p.name == settings.default_profile)
        .filter(|p| p.remote.is_none() && p.shell.is_none())
        .map(|p| p.instance.clone())
        .filter(|i| !i.is_empty())
        .or_else(|| c.instances.first().cloned());
    Context::new(spec, default)
}

fn run_all(c: &mut Ctx<'_>) {
    if let Some(spec) = c.state.doctor.spec().cloned() {
        let ctx = context(c, &spec);
        c.state.doctor.run_all(ctx);
    }
}

/// The light, the count and the time, and the page's two buttons. Above the
/// scroll, so they stay in view down a long list.
pub(super) fn top(ui: &mut Ui, c: &mut Ctx<'_>) {
    let path = c.settings.doctor_file();
    c.state.doctor.load(path.as_deref());
    if c.state.doctor.spec().is_some() && !c.state.doctor.has_run() {
        run_all(c);
    }

    let doctor = &c.state.doctor;
    let mut rerun = false;
    let mut copy = false;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if doctor.spec().is_some() {
            let word = if doctor.is_running() {
                tr("Checking...")
            } else {
                match doctor.overall() {
                    Some(status) => overall_word(status),
                    None => tr("Not checked"),
                }
            };
            lamp(ui, light(doctor));
            ui.label(RichText::new(word).strong());
            if !doctor.is_running() && doctor.checked_at.is_some() {
                let issues = doctor.issues();
                ui.label(
                    RichText::new(if issues == 1 {
                        tr("1 issue").to_string()
                    } else {
                        tr1("{} issues", &issues.to_string())
                    })
                    .weak(),
                );
            }
            if let Some(at) = doctor.checked_at {
                ui.label(
                    RichText::new(tr1("Checked {}", &at.format("%d/%m/%Y %H:%M").to_string()))
                        .weak(),
                );
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if doctor.spec().is_some() || doctor.load_error().is_some() {
                rerun = ui
                    .add_enabled_ui(!doctor.is_running(), |ui| {
                        prefs::button(ui, tr("Re-run checks"))
                    })
                    .inner
                    .tip(tr("Reads the checks file again and runs every check."))
                    .clicked();
            }
            if doctor.spec().is_some() && doctor.checked_at.is_some() {
                let label = if c.state.doctor_page.copied {
                    tr("Copied")
                } else {
                    tr("Copy report")
                };
                copy = prefs::button(ui, label)
                    .tip(tr("Every result as text, for a ticket or a chat. Your home folder and user name are left out."))
                    .clicked();
            }
        });
    });
    if let Some(error) = doctor.load_error() {
        ui.label(
            RichText::new(tr1("The checks file could not be read: {}", error))
                .color(colour(Status::Error)),
        );
    } else if doctor.path().is_none() {
        ui.label(RichText::new(tr("No checks file yet. Choose one under Checks file below, or ship org-doctor.xml beside the program.")).weak());
    }
    ui.add_space(4.0);

    if copy {
        let report = c.state.doctor.report();
        ui.ctx().copy_text(report);
        c.state.doctor_page.copied = true;
    }
    if rerun {
        c.state.doctor_page.copied = false;
        c.state.doctor_page.fix_error = None;
        c.state.doctor.reload(path.as_deref());
        run_all(c);
    }
}

/// Every check, a card per group.
fn checks(ui: &mut Ui, c: &mut Ctx<'_>) {
    let Some(spec) = c.state.doctor.spec().cloned() else {
        return;
    };
    let os = spec::Os::current();
    let mut actions: Vec<Action> = Vec::new();
    {
        let doctor = &c.state.doctor;
        let page = &c.state.doctor_page;
        for (index, group) in spec.groups().into_iter().enumerate() {
            let heading = match group {
                spec::RUNTIMES_GROUP => tr("Runtimes"),
                "" => tr("Checks"),
                named => named,
            };
            prefs::titled_card(ui, Some(heading), true, ("doctor", index), |card| {
                for check in spec.checks.iter().filter(|ch| ch.group == group) {
                    check_row(card, &spec, check, doctor, page, os, &mut actions);
                }
            });
        }
    }
    for action in actions {
        act(c, &spec, action);
    }
}

fn check_row(
    card: &mut Card<'_>,
    spec: &spec::Spec,
    check: &Check,
    doctor: &Doctor,
    page: &DoctorPageState,
    os: spec::Os,
    actions: &mut Vec<Action>,
) {
    let result = doctor.result(&check.id);
    let checking = doctor.is_checking(&check.id);
    let summary = match (checking, result) {
        (true, _) => tr("Checking...").to_string(),
        (false, Some(r)) if r.summary.is_empty() => status_word(r.status).to_string(),
        (false, Some(r)) => r.summary.clone(),
        (false, None) => tr("Not checked").to_string(),
    };
    let edge = match (checking, result) {
        (true, _) | (false, None) => WAITING,
        (false, Some(r)) => colour(r.status),
    };
    let shown = card.row(
        Row::new(&check.title).subtitle(summary.as_str()).link(true),
        |ui| {
            if checking {
                ui.add(egui::Spinner::new());
            } else if let Some(r) = result {
                chip(ui, r.status);
            }
        },
    );
    // The light down the row's edge, as Clawd draws its own.
    card.ui().painter().rect_filled(
        Rect::from_min_max(
            shown.rect.left_top() + egui::vec2(1.0, 5.0),
            egui::pos2(shown.rect.left() + 4.0, shown.rect.bottom() - 5.0),
        ),
        Rounding::same(2.0),
        edge,
    );
    if shown.label.clicked() {
        actions.push(Action::Toggle(check.id.clone()));
    }
    let open = page.expanded.contains(&check.id);
    card.reveal(("doctor-check", &check.id), open, |card| {
        card.custom(|ui| detail(ui, spec, check, doctor, page, os, actions));
    });
}

/// What an open check says: what it is for, what it found at length, and the
/// buttons to run it again or fix it.
fn detail(
    ui: &mut Ui,
    spec: &spec::Spec,
    check: &Check,
    doctor: &Doctor,
    page: &DoctorPageState,
    os: spec::Os,
    actions: &mut Vec<Action>,
) {
    let objective = if check.builtin {
        spec::runtime_objective(&check.id)
            .map(tr)
            .unwrap_or_default()
    } else {
        check.objective.as_str()
    };
    if !objective.is_empty() {
        prefs::explanation(ui, objective);
    }
    let result = doctor.result(&check.id);
    if let Some(r) = result.filter(|r| !r.detail.trim().is_empty()) {
        ui.add(egui::Label::new(RichText::new(r.detail.trim_end()).monospace().weak()).wrap());
    }
    let needs: Vec<&str> = check
        .requires
        .iter()
        .filter_map(|r| spec.check(&r.check).map(|c| c.title.as_str()))
        .collect();
    if !needs.is_empty() {
        prefs::explanation(ui, &tr1("Requires: {}", &needs.join(", ")));
    }
    if let Some(r) = result {
        prefs::explanation(
            ui,
            &tr1("Took {} s", &format!("{:.1}", r.took.as_secs_f32())),
        );
    }

    let fix = check
        .fix
        .as_ref()
        .filter(|_| check.fix_command(os).is_some());
    let broken = result.is_some_and(|r| is_issue(check, r.status));
    ui.horizontal_wrapped(|ui| {
        let idle = !doctor.is_running();
        if ui
            .add_enabled_ui(idle, |ui| prefs::button(ui, tr("Check again")))
            .inner
            .clicked()
        {
            actions.push(Action::RunOne(check.id.clone()));
        }
        let Some(fix) = fix else {
            return;
        };
        let title = if fix.title.is_empty() {
            tr("Fix")
        } else {
            fix.title.as_str()
        };
        if page.confirming.as_deref() == Some(check.id.as_str()) {
            ui.label(tr1("Run \u{201c}{}\u{201d} in a new tab?", title));
            if prefs::button(ui, tr("Run fix")).clicked() {
                actions.push(Action::Fix(check.id.clone()));
            }
            if prefs::button(ui, tr("Cancel")).clicked() {
                actions.push(Action::CancelFix);
            }
        } else if broken
            && ui
                .add_enabled_ui(idle, |ui| prefs::button(ui, title))
                .inner
                .tip(tr("Runs the fix in a tab of its own, in view, so you can see what it does and answer what it asks."))
                .clicked()
        {
            actions.push(if fix.confirm {
                Action::AskFix(check.id.clone())
            } else {
                Action::Fix(check.id.clone())
            });
        }
    });

    if doctor.fixing.as_deref() == Some(check.id.as_str()) {
        ui.horizontal_wrapped(|ui| {
            prefs::explanation(
                ui,
                tr("The fix is running in its own tab. Once it has finished:"),
            );
            if ui
                .add_enabled_ui(!doctor.is_running(), |ui| prefs::button(ui, tr("Verify")))
                .inner
                .clicked()
            {
                actions.push(Action::Verify(check.id.clone()));
            }
        });
    }
    if let Some(verdict) = doctor.verdict.as_ref().filter(|v| v.check == check.id) {
        let (text, status) = if verdict.worked {
            (tr("The fix worked.").to_string(), Status::Ok)
        } else {
            (
                tr1(
                    "The fix did not put this right: {}",
                    &verdict.failing.join(", "),
                ),
                Status::Error,
            )
        };
        ui.label(RichText::new(text).color(colour(status)));
    }
    if let Some((id, error)) = &page.fix_error {
        if id == &check.id {
            ui.label(
                RichText::new(tr1("The fix could not be started: {}", error))
                    .color(colour(Status::Error)),
            );
        }
    }
}

fn act(c: &mut Ctx<'_>, spec: &spec::Spec, action: Action) {
    match action {
        Action::Toggle(id) => {
            let expanded = &mut c.state.doctor_page.expanded;
            if !expanded.remove(&id) {
                expanded.insert(id);
            }
        }
        Action::RunOne(id) => {
            let ctx = context(c, spec);
            c.state.doctor.run_one(&id, ctx);
        }
        Action::AskFix(id) => c.state.doctor_page.confirming = Some(id),
        Action::CancelFix => c.state.doctor_page.confirming = None,
        Action::Fix(id) => {
            c.state.doctor_page.confirming = None;
            let Some(check) = spec.check(&id) else {
                return;
            };
            let ctx = context(c, spec);
            match run::fix_profile(spec, check, &ctx, tr1("Fix: {}", &check.title)) {
                Ok(profile) => {
                    c.requests.push(UiRequest::OpenTab(Box::new(profile)));
                    c.state.doctor.fix_started(&id);
                    c.state.doctor_page.fix_error = None;
                }
                Err(e) => c.state.doctor_page.fix_error = Some((id, e.to_string())),
            }
        }
        Action::Verify(id) => {
            let ctx = context(c, spec);
            c.state.doctor.verify(&id, ctx);
        }
    }
}

/// The checks file's path, with a Browse button where there is a dialog.
fn path_row(card: &mut Card<'_>, c: &mut Ctx<'_>) {
    let bundled =
        c.settings.doctor_path.as_os_str().is_empty() && c.settings.doctor_file().is_some();
    let subtitle = if bundled {
        tr1(
            "Empty, so the file beside the program is used: {}",
            crate::config::DOCTOR_FILE,
        )
    } else {
        tr("An XML file of checks, on this machine or a share. Leave empty to use one shipped beside the program.").to_owned()
    };
    let picking = &mut c.state.doctor_page.picking;
    let settings = &mut *c.settings;
    let mut changed = false;
    card.row(Row::new(tr("Path")).subtitle(subtitle.as_str()), |ui| {
        if file_dialog::AVAILABLE
            && ui
                .add_enabled(picking.is_none(), prefs::button_widget(tr("Browse...")))
                .clicked()
        {
            *picking = Some(file_dialog::pick(
                ui.ctx(),
                tr("Choose the checks file"),
                file_dialog::Kind {
                    name: tr("Checks files"),
                    patterns: "*.xml",
                },
                &settings.doctor_path.display().to_string(),
            ));
        }
        let mut path = settings.doctor_path.display().to_string();
        if ui
            .add(egui::TextEdit::singleline(&mut path).desired_width(260.0))
            .changed()
        {
            settings.doctor_path = std::path::PathBuf::from(path.trim());
            changed = true;
        }
    });
    c.changed |= changed;
}

/// The answer of the checks file's dialog, whatever page is on screen when
/// it comes.
pub(super) fn collect_picked_file(ctx: &egui::Context, c: &mut Ctx<'_>) {
    if let Some(path) = file_dialog::collect(ctx, &mut c.state.doctor_page.picking) {
        c.settings.doctor_path = std::path::PathBuf::from(path);
        c.changed = true;
    }
}

// ---------------------------------------------------------------------------
// Colours and words
// ---------------------------------------------------------------------------

/// Not yet known: not run, or running.
const WAITING: Color32 = Color32::from_rgb(0x8e, 0x8e, 0x93);

/// Fixed rather than themed, like the sidebar's tiles: a status is read by
/// its colour, which only works if green is always green.
fn colour(status: Status) -> Color32 {
    match status {
        Status::Ok => Color32::from_rgb(0x30, 0xb8, 0x5a),
        Status::Warn => Color32::from_rgb(0xf0, 0x9a, 0x0a),
        Status::Error => Color32::from_rgb(0xe5, 0x48, 0x4d),
        Status::Unknown => Color32::from_rgb(0xa8, 0x6b, 0xd8),
        Status::Skip => WAITING,
        Status::Blocked => Color32::from_rgb(0x6b, 0x7a, 0x90),
    }
}

/// The Doctor's light in the sidebar and on its page, in the colour of how
/// things stand, lit like a traffic light: the sidebar says it before the
/// page is opened, as Clawd's dot does. Out until the checks have run, and
/// while they are running.
pub(super) fn light(doctor: &Doctor) -> (Color32, bool) {
    if doctor.is_running() {
        return (WAITING, false);
    }
    match doctor.overall() {
        Some(Status::Ok | Status::Skip) => (colour(Status::Ok), true),
        Some(status) => (colour(status), true),
        None => (WAITING, false),
    }
}

fn status_word(status: Status) -> &'static str {
    tr(match status {
        Status::Ok => "OK",
        Status::Warn => "Warning",
        Status::Error => "Error",
        Status::Skip => "Skipped",
        Status::Unknown => "Unknown",
        Status::Blocked => "Blocked",
    })
}

fn overall_word(status: Status) -> &'static str {
    match status {
        Status::Ok | Status::Skip => tr("OK"),
        other => status_word(other),
    }
}

fn lamp(ui: &mut Ui, (colour, lit): (Color32, bool)) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
    crate::ui::shading::status_light(
        ui.painter(),
        rect.center(),
        6.0,
        colour,
        lit,
        ui.visuals().dark_mode,
    );
}

/// The status word in a tinted pill, at the right of a row.
fn chip(ui: &mut Ui, status: Status) {
    let colour = colour(status);
    let text = RichText::new(status_word(status)).small().color(colour);
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    );
    let size = galley.size() + egui::vec2(14.0, 6.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(5.0), colour.gamma_multiply(0.16));
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, colour);
}
