//! Web addresses in what the far side printed.
//!
//! A development server announces itself with a line like `Local:
//! http://localhost:5173/`, and every other terminal opens that on Ctrl+click.
//! Nothing marks it as a link - it is plain text - so it is recognised here by
//! its shape, off the grid, the same way [`super::lineedit`] reads the prompt.
//!
//! Only `http` and `https`. The address is handed to the system to open, and a
//! `file:` link printed by a remote server could name a program on a share as
//! easily as a page.

use crate::term::Grid;

/// A web address on screen, and the cells it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub url: String,
    /// One `(line, start, end)` per grid line the address is on, `end`
    /// exclusive. More than one when a long address ran past the margin and
    /// the far side's own wrap carried it onto the next line.
    pub spans: Vec<(usize, usize, usize)>,
}

const SCHEMES: [&str; 2] = ["http://", "https://"];

/// How far either side of the pointer an address is looked for, in
/// characters. Past what any browser takes in an address bar.
const REACH: usize = 8192;

/// The web address that covers `col` on `line`, if any.
///
/// Read across soft wraps: a shell wraps a long address at the window's width,
/// and the half under the pointer is not an address on its own.
pub fn link_at(grid: &Grid, line: usize, col: usize) -> Option<Link> {
    grid.line(line)?;
    let wrapped = |l: usize| grid.line(l).is_some_and(|row| row.wrapped);
    let len = |l: usize| grid.line(l).map_or(0, |row| row.cells.len());

    // Only as far as an address can reach. This runs on every frame the
    // pointer is over the grid, and a minified file dumped in a shell is one
    // wrapped run as long as the scrollback: joining all of it cost 50 ms a
    // frame.
    let (mut first, mut behind) = (line, col);
    while first > 0 && wrapped(first - 1) && behind < REACH {
        first -= 1;
        behind += len(first);
    }
    let (mut last, mut ahead) = (line, len(line).saturating_sub(col));
    while wrapped(last) && grid.line(last + 1).is_some() && ahead < REACH {
        last += 1;
        ahead += len(last);
    }
    let cut_front = first > 0 && wrapped(first - 1);
    let cut_back = wrapped(last) && grid.line(last + 1).is_some();

    // The joined text, and the grid position each of its characters came from.
    // A wrapped row is taken to its full length - the wrap means the text went
    // on - and only the last one is cut back to what it holds.
    let mut chars = Vec::new();
    let mut at = Vec::new();
    let mut target = None;
    for l in first..=last {
        let row = grid.line(l)?;
        let width = if l < last || cut_back {
            row.cells.len()
        } else {
            row.used_width()
        };
        for (c, cell) in row.cells[..width].iter().enumerate() {
            if l == line && c == col {
                target = Some(chars.len());
            }
            chars.push(cell.ch);
            at.push((l, c));
        }
    }
    let (start, end) = url_bounds(&chars, target?)?;
    // Running into the edge of what was read means the address may go on past
    // it, and opening the part that was read would open somewhere else.
    if (cut_front && start == 0) || (cut_back && end == chars.len()) {
        return None;
    }

    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    for &(l, c) in &at[start..end] {
        match spans.last_mut() {
            Some((sl, _, e)) if *sl == l && *e == c => *e = c + 1,
            _ => spans.push((l, c, c + 1)),
        }
    }
    Some(Link {
        url: chars[start..end].iter().collect(),
        spans,
    })
}

/// The address in `chars` that covers `at`, as `start..end`.
fn url_bounds(chars: &[char], at: usize) -> Option<(usize, usize)> {
    if !is_url_char(*chars.get(at)?) {
        return None;
    }
    // The unbroken run of address characters around `at`: whatever address
    // covers it has to lie inside that run.
    let mut run_start = at;
    while run_start > 0 && is_url_char(chars[run_start - 1]) {
        run_start -= 1;
    }
    let mut run_end = at + 1;
    while run_end < chars.len() && is_url_char(chars[run_end]) {
        run_end += 1;
    }

    // An address starts at its scheme. Run back to front, the last scheme at
    // or before `at` is the one `at` belongs to - `see:http://a,http://b`
    // holds two, and the pointer is on one of them.
    let start = (run_start..=at)
        .rev()
        .find(|&i| scheme_at(&chars[..run_end], i).is_some())?;
    let scheme_len = scheme_at(&chars[..run_end], start)?;
    // Up to the next scheme, if two addresses are run together.
    let mut end = (start + 1..run_end)
        .find(|&i| scheme_at(&chars[..run_end], i).is_some())
        .unwrap_or(run_end);

    // Punctuation that ends the sentence, not the address: `see
    // http://host/a.` and `(at http://host/a)`. A closing bracket stays when
    // the address opened it, as in a wiki's `/Foo_(bar)`.
    while let Some(&last) = chars[start..end].last() {
        let open = match last {
            ')' => Some('('),
            ']' => Some('['),
            '}' => Some('{'),
            _ => None,
        };
        let trailing = match open {
            Some(open) => {
                let span = &chars[start..end];
                span.iter().filter(|&&c| c == last).count()
                    > span.iter().filter(|&&c| c == open).count()
            }
            None => matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | '*'),
        };
        if !trailing {
            break;
        }
        end -= 1;
    }

    // A scheme with nothing after it is not somewhere to go.
    (end > start + scheme_len && at < end).then_some((start, end))
}

/// The length of the scheme that starts an address at `i`, if one does.
fn scheme_at(chars: &[char], i: usize) -> Option<usize> {
    // `xhttp://` is not an address that starts at `h`.
    if i > 0 && chars[i - 1].is_ascii_alphanumeric() {
        return None;
    }
    SCHEMES
        .iter()
        .find(|scheme| {
            let mut rest = chars[i..].iter();
            scheme
                .chars()
                .all(|c| rest.next().is_some_and(|r| r.eq_ignore_ascii_case(&c)))
        })
        .map(|scheme| scheme.len())
}

/// What can appear in an address as printed. Quotes and angle brackets are
/// what an address is usually wrapped in. Past ASCII only letters and digits:
/// a path written in Portuguese (`/relatório`) is still the path, while a box
/// drawn around the address (`│ http://... │`) is not part of it.
fn is_url_char(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_graphic() && !matches!(c, '"' | '\'' | '<' | '>' | '`')
    } else {
        c.is_alphanumeric()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_with(cols: usize, lines: &[&str]) -> Grid {
        let mut grid = Grid::new(cols, lines.len().max(1), 100);
        for (r, line) in lines.iter().enumerate() {
            grid.screen[r].set_text(line);
        }
        grid
    }

    fn url(grid: &Grid, line: usize, col: usize) -> Option<String> {
        link_at(grid, line, col).map(|link| link.url)
    }

    #[test]
    fn an_address_is_found_from_anywhere_on_it() {
        let grid = grid_with(80, &["  Local:   http://localhost:5173/  "]);
        for col in 11..33 {
            assert_eq!(
                url(&grid, 0, col).as_deref(),
                Some("http://localhost:5173/"),
                "col {col}"
            );
        }
        assert_eq!(url(&grid, 0, 10), None);
        assert_eq!(url(&grid, 0, 33), None);
        assert_eq!(url(&grid, 0, 2), None);
    }

    #[test]
    fn the_cells_an_address_covers_are_reported_with_it() {
        let grid = grid_with(80, &["go https://example.com/a now"]);
        let link = link_at(&grid, 0, 5).unwrap();
        assert_eq!(link.spans, vec![(0, 3, 24)]);
    }

    #[test]
    fn punctuation_after_an_address_ends_the_sentence_not_the_address() {
        let grid = grid_with(
            80,
            &["see http://host/a. or (http://host/b), or http://w/Foo_(bar)"],
        );
        assert_eq!(url(&grid, 0, 6).as_deref(), Some("http://host/a"));
        assert_eq!(url(&grid, 0, 25).as_deref(), Some("http://host/b"));
        assert_eq!(url(&grid, 0, 45).as_deref(), Some("http://w/Foo_(bar)"));
    }

    #[test]
    fn quotes_and_frames_around_an_address_are_not_part_of_it() {
        let grid = grid_with(80, &["\"https://a.io/x\" │http://b.io│"]);
        assert_eq!(url(&grid, 0, 3).as_deref(), Some("https://a.io/x"));
        assert_eq!(url(&grid, 0, 22).as_deref(), Some("http://b.io"));
    }

    #[test]
    fn two_addresses_run_together_are_two_addresses() {
        let grid = grid_with(80, &["http://a.io,http://b.io"]);
        assert_eq!(url(&grid, 0, 2).as_deref(), Some("http://a.io"));
        assert_eq!(url(&grid, 0, 14).as_deref(), Some("http://b.io"));
    }

    #[test]
    fn only_web_addresses_count() {
        let grid = grid_with(
            80,
            &["file:///C:/x.exe ftp://h/ xhttp://h/ http:// mailto:a@b"],
        );
        for col in 0..55 {
            assert_eq!(url(&grid, 0, col), None, "col {col}");
        }
    }

    #[test]
    fn an_address_wrapped_past_the_margin_is_read_as_one() {
        let mut grid = grid_with(10, &["at http://", "host/long/", "path done"]);
        grid.screen[0].wrapped = true;
        grid.screen[1].wrapped = true;

        for (line, col) in [(0, 4), (1, 2), (2, 1)] {
            let link = link_at(&grid, line, col).expect("an address");
            assert_eq!(link.url, "http://host/long/path");
            assert_eq!(link.spans, vec![(0, 3, 10), (1, 0, 10), (2, 0, 4)]);
        }
        assert_eq!(url(&grid, 2, 6), None);
    }

    #[test]
    fn a_hard_line_break_ends_an_address() {
        let grid = grid_with(10, &["at http://", "host/long/"]);
        assert_eq!(url(&grid, 0, 4), None);
        assert_eq!(url(&grid, 1, 2), None);
    }

    #[test]
    fn a_path_with_accents_is_opened_whole() {
        let grid = grid_with(80, &["veja http://host/relatório/ação ok"]);
        assert_eq!(
            url(&grid, 0, 8).as_deref(),
            Some("http://host/relatório/ação")
        );
    }

    #[test]
    fn a_scheme_in_capitals_is_still_a_scheme() {
        let grid = grid_with(80, &["HTTPS://HOST/A"]);
        assert_eq!(url(&grid, 0, 3).as_deref(), Some("HTTPS://HOST/A"));
    }

    /// A shell's wrapped run can be as long as the scrollback. Only what an
    /// address can span is read, so an address inside it is found as before,
    /// and one running past the reach is refused rather than opened in part.
    #[test]
    fn a_wrapped_run_is_read_only_as_far_as_an_address_can_reach() {
        let cols = 100;
        let rows = 3 * REACH / cols;
        let mut grid = Grid::new(cols, rows, 0);
        for r in 0..rows {
            grid.screen[r].set_text(&"a".repeat(cols));
            grid.screen[r].wrapped = r + 1 < rows;
        }
        let middle = rows / 2;
        grid.screen[middle].set_text(&format!("{:<1$}", " http://x.io/ ", cols));
        assert_eq!(url(&grid, middle, 5).as_deref(), Some("http://x.io/"));

        // An address with no end in sight.
        grid.screen[middle].set_text(&format!("{:a<1$}", " http://x.io/", cols));
        assert_eq!(url(&grid, middle, 5), None);
    }
}
