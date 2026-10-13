//! The README's pictures, recorded from the scenes above rather than drawn a
//! second time. Each scene is run through egui a frame at a time, and what
//! egui would have painted is written out as SVG, one group per frame, shown
//! in turn by SMIL timing - which, unlike script, still runs where GitHub
//! embeds the file as an image. Regenerate after changing a scene:
//!
//! ```text
//! cargo test --lib readme_pictures -- --ignored
//! ```
//!
//! Run it on its own: it sets the interface language, which every other test
//! reading `tr` would race with.

use std::fmt::Write as _;
use std::path::Path;

use egui::epaint::{ColorMode, PathStroke, Shape};
use egui::{Color32, FontFamily, Painter, Pos2, Rect, Vec2};

use super::*;
use crate::i18n::{self, Lang};

/// The size the settings draw a scene at, which the README scales to fit.
const SIZE: Vec2 = Vec2::new(420.0, 193.0);
/// Frames a second: smooth enough for a pointer gliding across a screen.
const FPS: f64 = 15.0;

type Draw = fn(&Painter, Rect, &Ink, f32);

/// The pictures the README shows, by file name, with the scene in each.
fn pictures() -> [(&'static str, Draw, f64, f32); 5] {
    [
        (
            "global-tooltip",
            global_tooltip,
            Scene::GlobalTooltip.period(),
            Scene::GlobalTooltip.still_at(),
        ),
        (
            "autocomplete",
            autocomplete,
            Scene::Autocomplete.period(),
            Scene::Autocomplete.still_at(),
        ),
        ("claude", claude, 8.0, 0.9),
        (
            "macros",
            macros,
            Scene::Macros.period(),
            Scene::Macros.still_at(),
        ),
        (
            "shells",
            shells,
            Scene::Shells.period(),
            Scene::Shells.still_at(),
        ),
    ]
}

/// An error on screen, Analyze with Claude from the right-click menu, and the
/// tab that opens already knowing about it. The settings have no page for it,
/// so it lives only here.
fn claude(painter: &Painter, rect: Rect, ink: &Ink, t: f32) {
    let opened = t > 0.42;
    let tabs: &[&str] = if opened {
        &["IRIS", "Claude"]
    } else {
        &["IRIS"]
    };
    let screen = window(painter, rect, ink, tabs, usize::from(opened));
    let mono = Mono::for_screen(painter, screen);
    let font = FontId::proportional(mono.font.size);
    if !opened {
        let error = Color32::from_rgb(235, 105, 95);
        mono.text(painter, mono.at(screen, 0, 0), "USER>", ink.weak);
        mono.text(
            painter,
            mono.at(screen, 5, 0),
            "do ##class(Utils.Base).Run()",
            ink.typed,
        );
        mono.text(
            painter,
            mono.at(screen, 0, 1),
            "<UNDEFINED>zRun+7^Utils.Base.1 *args",
            error,
        );
        mono.text(painter, mono.at(screen, 0, 2), "USER>", ink.weak);
        let click = mono.at(screen, 14, 1) + Vec2::new(0.0, mono.row * 0.5);
        let tip = lerp(
            screen.right_bottom() - Vec2::new(20.0, 14.0),
            click,
            ease(t, 0.03, 0.16),
        );
        let menu = during(t, 0.18, 0.43);
        if menu > 0.0 {
            popup(
                painter,
                click + Vec2::new(4.0, 4.0),
                &[tr("Copy"), tr("Analyze with Claude")],
                (t > 0.27).then_some(1),
                &font,
                ink,
                menu,
            );
        }
        pointer(
            painter,
            tip,
            ink,
            (0.16..0.19).contains(&t) || (0.36..0.4).contains(&t),
        );
        return;
    }
    let (question, answer) = match i18n::language() {
        Lang::PtBr => (
            "por que args está indefinido?",
            [
                "zRun+7 lê args, mas Run foi chamado",
                "sem ele. Passe por referência:",
                "do ##class(Utils.Base).Run(.args)",
            ],
        ),
        Lang::En => (
            "why is args undefined?",
            [
                "zRun+7 reads args, but Run was called",
                "without it. Pass it by reference:",
                "do ##class(Utils.Base).Run(.args)",
            ],
        ),
    };
    mono.text(painter, mono.at(screen, 0, 0), ">", ink.accent);
    let shown: String = question
        .chars()
        .take(typed(t, 0.48, 0.66, question.chars().count()))
        .collect();
    mono.text(painter, mono.at(screen, 2, 0), &shown, ink.typed);
    for (i, line) in answer.iter().enumerate() {
        let fade = ease(t, 0.72 + i as f32 * 0.05, 0.76 + i as f32 * 0.05);
        if fade > 0.0 {
            let colour = if i == 2 { ink.typed } else { ink.text };
            if i == 0 {
                mono.text(painter, mono.at(screen, 0, 2), "●", faded(ink.accent, fade));
            }
            mono.text(
                painter,
                mono.at(screen, 2, 2 + i),
                line,
                faded(colour, fade),
            );
        }
    }
}

/// What egui paints for `draw` at `t`, as SVG elements.
fn frame(ctx: &egui::Context, draw: Draw, t: f32) -> String {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
        ..Default::default()
    };
    let out = ctx.run(input, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let rect = Rect::from_min_size(Pos2::ZERO, SIZE);
                let ink = Ink::of(ui);
                draw(&ui.painter_at(rect), rect, &ink, t);
            });
    });
    let mut svg = String::new();
    for clipped in out.shapes {
        write_shape(&mut svg, &clipped.shape);
    }
    svg
}

/// ` fill="#rrggbb" fill-opacity=".."`, or nothing for no colour at all.
fn paint(attr: &str, colour: Color32) -> Option<String> {
    let [r, g, b, a] = colour.to_srgba_unmultiplied();
    if a == 0 {
        return None;
    }
    let mut out = format!(" {attr}=\"#{r:02x}{g:02x}{b:02x}\"");
    if a < 255 {
        let _ = write!(out, " {attr}-opacity=\"{:.3}\"", f32::from(a) / 255.0);
    }
    Some(out)
}

fn stroke(width: f32, colour: Color32) -> String {
    match paint("stroke", colour) {
        Some(p) if width > 0.0 => format!("{p} stroke-width=\"{width}\""),
        _ => String::new(),
    }
}

fn path_stroke(s: &PathStroke) -> String {
    match s.color {
        ColorMode::Solid(c) => stroke(s.width, c),
        ColorMode::UV(_) => String::new(),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn write_shape(svg: &mut String, shape: &Shape) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| write_shape(svg, s)),
        Shape::Rect(r) => {
            let fill = paint("fill", r.fill).unwrap_or_else(|| " fill=\"none\"".into());
            let line = stroke(r.stroke.width, r.stroke.color);
            let (x, y, w, h) = (r.rect.left(), r.rect.top(), r.rect.width(), r.rect.height());
            let k = r.rounding;
            if k.nw == k.ne && k.ne == k.sw && k.sw == k.se {
                let _ = writeln!(
                    svg,
                    "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{}\"{fill}{line}/>",
                    k.nw
                );
            } else {
                let (r0, r1) = (x + w, y + h);
                let _ = writeln!(
                    svg,
                    "<path d=\"M{} {y}H{}Q{r0} {y} {r0} {}V{}Q{r0} {r1} {} {r1}H{}Q{x} {r1} {x} {}V{}Q{x} {y} {} {y}Z\"{fill}{line}/>",
                    x + k.nw,
                    r0 - k.ne,
                    y + k.ne,
                    r1 - k.se,
                    r0 - k.se,
                    x + k.sw,
                    r1 - k.sw,
                    y + k.nw,
                    x + k.nw,
                );
            }
        }
        Shape::Circle(c) => {
            let fill = paint("fill", c.fill).unwrap_or_else(|| " fill=\"none\"".into());
            let _ = writeln!(
                svg,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"{fill}{}/>",
                c.center.x,
                c.center.y,
                c.radius,
                stroke(c.stroke.width, c.stroke.color)
            );
        }
        Shape::LineSegment { points, stroke: s } => {
            let _ = writeln!(
                svg,
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{}/>",
                points[0].x,
                points[0].y,
                points[1].x,
                points[1].y,
                path_stroke(s)
            );
        }
        Shape::Path(p) => {
            let points: Vec<String> = p
                .points
                .iter()
                .map(|q| format!("{},{}", q.x, q.y))
                .collect();
            let fill = paint("fill", p.fill).unwrap_or_else(|| " fill=\"none\"".into());
            let line = path_stroke(&p.stroke);
            let tag = if p.closed { "polygon" } else { "polyline" };
            let _ = writeln!(svg, "<{tag} points=\"{}\"{fill}{line}/>", points.join(" "));
        }
        Shape::Text(text) => {
            let galley = &text.galley;
            for row in &galley.rows {
                // One run per stretch of glyphs in the same format, each held to
                // the width egui gave it: a browser's monospace is not egui's,
                // and without the width the columns would drift.
                let mut start = 0;
                while start < row.glyphs.len() {
                    let section = row.glyphs[start].section_index;
                    let end = row.glyphs[start..]
                        .iter()
                        .position(|g| g.section_index != section)
                        .map_or(row.glyphs.len(), |n| start + n);
                    let glyphs = &row.glyphs[start..end];
                    start = end;
                    let content: String = glyphs.iter().map(|g| g.chr).collect();
                    if content.trim().is_empty() {
                        continue;
                    }
                    let format = &galley.job.sections[section as usize].format;
                    let mut colour = text.override_text_color.unwrap_or(format.color);
                    if colour == Color32::PLACEHOLDER {
                        colour = text.fallback_color;
                    }
                    let colour = colour.gamma_multiply(text.opacity_factor.clamp(0.0, 1.0));
                    let Some(fill) = paint("fill", colour) else {
                        continue;
                    };
                    let family = match format.font_id.family {
                        FontFamily::Monospace => "mono",
                        _ => "sans",
                    };
                    let first = &glyphs[0];
                    let width = glyphs[glyphs.len() - 1].max_x() - first.pos.x;
                    let _ = writeln!(
                        svg,
                        "<text class=\"{family}\" x=\"{}\" y=\"{}\" font-size=\"{}\" textLength=\"{width}\" lengthAdjust=\"spacingAndGlyphs\"{fill}>{}</text>",
                        text.pos.x + first.pos.x,
                        text.pos.y + first.pos.y,
                        format.font_id.size,
                        escape(&content)
                    );
                }
            }
        }
        _ => {}
    }
}

/// The whole picture: every distinct frame, each shown for its stretch of the
/// loop. The one at `still` is also shown where nothing animates, as a poster.
fn picture(draw: Draw, period: f64, still: f32, label: &str) -> String {
    let ctx = egui::Context::default();
    ctx.set_visuals(egui::Visuals::dark());
    // The first frame lays out the fonts; nothing drawn in it is kept.
    let _ = frame(&ctx, draw, 0.0);
    let count = (period * FPS).round() as usize;
    let mut runs: Vec<(usize, usize, String)> = Vec::new();
    for i in 0..count {
        let body = frame(&ctx, draw, i as f32 / count as f32);
        match runs.last_mut() {
            Some((_, end, last)) if *last == body => *end = i + 1,
            _ => runs.push((i, i + 1, body)),
        }
    }
    let poster = ((still * count as f32) as usize).min(count - 1);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w2}\" height=\"{h2}\" role=\"img\" aria-label=\"{label}\">\n\
         <style>text{{white-space:pre;dominant-baseline:alphabetic}}\
         .mono{{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,'Liberation Mono',monospace}}\
         .sans{{font-family:system-ui,-apple-system,'Segoe UI',Roboto,sans-serif}}</style>\n",
        w = SIZE.x,
        h = SIZE.y,
        w2 = SIZE.x * 2.0,
        h2 = SIZE.y * 2.0,
        label = escape(label).replace('"', "&quot;"),
    );
    for (from, to, body) in &runs {
        let shown = (*from..*to).contains(&poster);
        let _ = write!(
            svg,
            "<g visibility=\"{}\">",
            if shown { "visible" } else { "hidden" }
        );
        if runs.len() > 1 {
            let (a, b) = (*from as f64 / count as f64, *to as f64 / count as f64);
            let (values, times) = if *from == 0 {
                ("visible;hidden".to_owned(), format!("0;{b:.4}"))
            } else if *to == count {
                ("hidden;visible".to_owned(), format!("0;{a:.4}"))
            } else {
                (
                    "hidden;visible;hidden".to_owned(),
                    format!("0;{a:.4};{b:.4}"),
                )
            };
            let _ = write!(
                svg,
                "<animate attributeName=\"visibility\" calcMode=\"discrete\" values=\"{values}\" keyTimes=\"{times}\" dur=\"{period}s\" repeatCount=\"indefinite\"/>"
            );
        }
        let _ = writeln!(svg, "\n{body}</g>");
    }
    svg.push_str("</svg>\n");
    svg
}

/// The alt text of each picture, as the README has it.
fn label(name: &str, lang: Lang) -> &'static str {
    match (name, lang) {
        ("global-tooltip", Lang::En) => "The pointer over a piece of a zwrite line; a tooltip names the property and its values",
        ("global-tooltip", Lang::PtBr) => "O ponteiro sobre uma peça de uma linha de zwrite; um tooltip mostra a propriedade e seus valores",
        ("autocomplete", Lang::En) => "Typing inside ^mtemp( narrows a list of the subscripts that exist",
        ("autocomplete", Lang::PtBr) => "Digitando dentro de ^mtemp( a lista de subscritos existentes vai se estreitando",
        ("claude", Lang::En) => "An error, Analyze with Claude from the right-click menu, and a Claude tab answering about it",
        ("claude", Lang::PtBr) => "Um erro, Analisar com o Claude no menu do botão direito, e uma aba do Claude respondendo sobre ele",
        ("macros", Lang::En) => "A macro chosen from the right-click menu, its parameter filled in, and the line sent",
        ("macros", Lang::PtBr) => "Uma macro escolhida no menu do botão direito, o parâmetro preenchido e a linha enviada",
        (_, Lang::En) => "IRIS, PowerShell and Git Bash opening in tabs of one window",
        (_, Lang::PtBr) => "IRIS, PowerShell e Git Bash abrindo em abas da mesma janela",
    }
}

#[test]
#[ignore = "writes docs/images; run on its own, see the module comment"]
fn readme_pictures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/images");
    let before = i18n::language();
    // Portuguese beside README.md, English beside README.en.md.
    for (lang, dir) in [(Lang::PtBr, root.clone()), (Lang::En, root.join("en"))] {
        i18n::set_language(lang);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, draw, period, still) in pictures() {
            let svg = picture(draw, period, still, label(name, lang));
            std::fs::write(dir.join(format!("{name}.svg")), svg).unwrap();
        }
    }
    i18n::set_language(before);
}

#[test]
fn a_recorded_picture_has_frames_that_take_turns() {
    let svg = picture(shells, 1.0, 0.5, "x");
    assert!(svg.starts_with("<svg") && svg.ends_with("</svg>\n"));
    assert!(svg.contains("<animate"), "it moves");
    assert_eq!(
        svg.matches("visibility=\"visible\">").count(),
        1,
        "one poster"
    );
    assert!(
        svg.contains("PowerShell") || svg.contains("IRIS"),
        "with its words"
    );
}

#[test]
fn the_claude_scene_draws_at_every_point_of_its_loop() {
    for step in 0..=40 {
        let _ = frame(&egui::Context::default(), claude, step as f32 / 40.0);
    }
}
