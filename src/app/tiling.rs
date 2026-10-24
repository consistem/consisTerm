//! How a tab's panes share its room: a binary tree of splits over them.
//!
//! The way a tiling window manager lays out windows, scaled down to the panes
//! of one tab: every split halves a region, left/right or top/bottom, at a
//! ratio of its own, and either half may be split again. A pane is a leaf,
//! named by its [`Pane`] number; the tree holds no sessions, only where they
//! go, which is what lets everything here be tested without a terminal.
//!
//! Binary rather than n-ary, as bspwm has it rather than i3: one ratio per
//! divider means one divider is one drag, and "split this pane" is always the
//! same edit wherever the pane sits.

use egui::{Pos2, Rect, Vec2};

use super::layout::{split_extent, SPLIT_DIVIDER};

/// Which way the second half of a split sits from the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDir {
    /// Side by side: the second half on the right.
    Right,
    /// One above the other: the second half below.
    Bottom,
}

impl SplitDir {
    pub fn flipped(self) -> SplitDir {
        match self {
            SplitDir::Right => SplitDir::Bottom,
            SplitDir::Bottom => SplitDir::Right,
        }
    }
}

/// A direction on screen, for moving the focus or a pane, and for the side of
/// a pane something is docked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Left,
    Right,
    Up,
    Down,
}

impl Toward {
    /// The split that puts something on this side of a pane, and whether the
    /// new part is the split's first half.
    pub fn as_split(self) -> (SplitDir, bool) {
        match self {
            Toward::Left => (SplitDir::Right, true),
            Toward::Right => (SplitDir::Right, false),
            Toward::Up => (SplitDir::Bottom, true),
            Toward::Down => (SplitDir::Bottom, false),
        }
    }
}

/// One session of a tab. `Pane(0)` is the tab's own session, the only one a
/// tab that is not split has; the others are numbered on from 1 in the order
/// they were opened, which is also the order they are kept in.
///
/// The number shown in the strip is one more, so the panes the user sees are
/// `1:`, `2:`, `3:` and so on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pane(pub usize);

impl Pane {
    pub const FIRST: Pane = Pane(0);

    /// What the strip entry calls this pane.
    pub fn number(self) -> usize {
        self.0 + 1
    }
}

/// The layout of a split tab.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Leaf(Pane),
    Split {
        dir: SplitDir,
        /// How much of the room the first half gets, 0 to 1. Turned into a
        /// size by `split_extent`, which keeps either half from being
        /// squeezed away whatever this says.
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

/// Where one divider was drawn, and what dragging it changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Divider {
    /// Which split, counted through the tree first-half first. Stable for as
    /// long as the tree is not edited, which is the length of a drag.
    pub id: usize,
    pub dir: SplitDir,
    pub rect: Rect,
    /// The room the two halves share, divider and spacing taken off.
    pub usable: f32,
    pub ratio: f32,
}

/// What laying a tree out in a rectangle produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tiles {
    pub panes: Vec<(Pane, Rect)>,
    pub dividers: Vec<Divider>,
}

impl Node {
    pub fn split(dir: SplitDir, first: Node, second: Node) -> Node {
        Node::Split {
            dir,
            ratio: 0.5,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Every pane in the tree, left to right and top to bottom.
    pub fn panes(&self) -> Vec<Pane> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<Pane>) {
        match self {
            Node::Leaf(pane) => out.push(*pane),
            Node::Split { first, second, .. } => {
                first.collect(out);
                second.collect(out);
            }
        }
    }

    pub fn contains(&self, pane: Pane) -> bool {
        match self {
            Node::Leaf(p) => *p == pane,
            Node::Split { first, second, .. } => first.contains(pane) || second.contains(pane),
        }
    }

    /// The first pane of this part of the tree - the one that inherits the
    /// keyboard when the pane beside it goes.
    pub fn first_pane(&self) -> Pane {
        match self {
            Node::Leaf(pane) => *pane,
            Node::Split { first, .. } => first.first_pane(),
        }
    }

    /// Puts `part` against the `side` of `at`, splitting the room `at` had
    /// between them. False, and nothing changed, when `at` is not in the tree.
    pub fn dock(&mut self, at: Pane, side: Toward, part: Node) -> bool {
        let Some(leaf) = self.leaf_mut(at) else {
            return false;
        };
        let (dir, part_first) = side.as_split();
        let old = Node::Leaf(at);
        *leaf = if part_first {
            Node::split(dir, part, old)
        } else {
            Node::split(dir, old, part)
        };
        true
    }

    fn leaf_mut(&mut self, at: Pane) -> Option<&mut Node> {
        match self {
            Node::Leaf(pane) if *pane == at => Some(self),
            Node::Leaf(_) => None,
            Node::Split { first, second, .. } => {
                // Asked first rather than borrowed: the borrow checker will
                // not hand back a borrow from one arm and then try the other.
                if first.contains(at) {
                    first.leaf_mut(at)
                } else {
                    second.leaf_mut(at)
                }
            }
        }
    }

    /// Takes `gone` out of the tree, the half beside it taking over the room
    /// the split gave them both, and renumbers every pane after it down by one
    /// - which is what the list of sessions does when one is removed from it.
    ///
    /// Returns the pane that took over the room, by its new number, or `None`
    /// when `gone` is not in the tree or is all of it: a tree cannot be left
    /// with no pane, and a tab with one session is not split.
    pub fn remove(&mut self, gone: Pane) -> Option<Pane> {
        let heir = self.cut(gone)?;
        self.renumber(|pane| if pane > gone { Pane(pane.0 - 1) } else { pane });
        Some(if heir > gone { Pane(heir.0 - 1) } else { heir })
    }

    fn cut(&mut self, gone: Pane) -> Option<Pane> {
        let Node::Split { first, second, .. } = self else {
            return None;
        };
        let keep = if **first == Node::Leaf(gone) {
            std::mem::replace(second.as_mut(), Node::Leaf(Pane(0)))
        } else if **second == Node::Leaf(gone) {
            std::mem::replace(first.as_mut(), Node::Leaf(Pane(0)))
        } else if first.contains(gone) {
            return first.cut(gone);
        } else if second.contains(gone) {
            return second.cut(gone);
        } else {
            return None;
        };
        let heir = keep.first_pane();
        *self = keep;
        Some(heir)
    }

    /// Moves pane `from` against the `side` of pane `onto`, the room it
    /// leaves going to the pane beside it. Its number is kept: only its place
    /// changes. False, and nothing changed, when either is not in the tree or
    /// they are the same pane.
    pub fn relocate(&mut self, from: Pane, onto: Pane, side: Toward) -> bool {
        if from == onto || !self.contains(from) || !self.contains(onto) {
            return false;
        }
        if self.cut(from).is_none() {
            return false;
        }
        self.dock(onto, side, Node::Leaf(from))
    }

    /// Relabels every pane through `f`.
    pub fn renumber(&mut self, f: impl Fn(Pane) -> Pane + Copy) {
        match self {
            Node::Leaf(pane) => *pane = f(*pane),
            Node::Split { first, second, .. } => {
                first.renumber(f);
                second.renumber(f);
            }
        }
    }

    /// Trades the places of two panes, leaving the shape of the tree alone.
    pub fn swap(&mut self, a: Pane, b: Pane) {
        self.renumber(|pane| {
            if pane == a {
                b
            } else if pane == b {
                a
            } else {
                pane
            }
        });
    }

    /// Turns the split `pane` sits directly in from side by side to one above
    /// the other, or back. False when the pane is the whole tree.
    pub fn flip_around(&mut self, pane: Pane) -> bool {
        match self {
            Node::Leaf(_) => false,
            Node::Split {
                dir, first, second, ..
            } => {
                if **first == Node::Leaf(pane) || **second == Node::Leaf(pane) {
                    *dir = dir.flipped();
                    true
                } else if first.contains(pane) {
                    first.flip_around(pane)
                } else {
                    second.flip_around(pane)
                }
            }
        }
    }

    /// Sets the ratio of the split [`Divider::id`] names.
    pub fn set_ratio(&mut self, id: usize, to: f32) {
        let mut next = 0;
        self.set_ratio_counting(id, to, &mut next);
    }

    fn set_ratio_counting(&mut self, id: usize, to: f32, next: &mut usize) {
        if let Node::Split {
            ratio,
            first,
            second,
            ..
        } = self
        {
            if *next == id {
                *ratio = to;
                // Still counted on: the ids below are spent either way.
            }
            *next += 1;
            first.set_ratio_counting(id, to, next);
            second.set_ratio_counting(id, to, next);
        }
    }

    /// Lays the tree out in `room`, with `spacing` either side of every
    /// divider.
    ///
    /// The halves are given exact sizes, so what a divider and its spacing
    /// take comes off the room first: a share that did not fit would be
    /// clipped at the window's edge.
    pub fn tile(&self, room: Rect, spacing: Vec2) -> Tiles {
        let mut tiles = Tiles::default();
        let mut next = 0;
        self.tile_into(room, spacing, &mut tiles, &mut next);
        tiles
    }

    fn tile_into(&self, room: Rect, spacing: Vec2, tiles: &mut Tiles, next: &mut usize) {
        match self {
            Node::Leaf(pane) => tiles.panes.push((*pane, room)),
            Node::Split {
                dir,
                ratio,
                first,
                second,
            } => {
                let id = *next;
                *next += 1;
                let (a, divider, b, usable) = halves(room, *dir, *ratio, spacing);
                tiles.dividers.push(Divider {
                    id,
                    dir: *dir,
                    rect: divider,
                    usable,
                    ratio: *ratio,
                });
                first.tile_into(a, spacing, tiles, next);
                second.tile_into(b, spacing, tiles, next);
            }
        }
    }

    /// The pane next to `from` in the direction `toward`, if there is one.
    ///
    /// Worked out on screen rather than in the tree, which is what the user is
    /// looking at: the pane to the left is the one whose right edge meets this
    /// one's left edge, wherever it hangs in the tree. Of several that meet it,
    /// the one alongside the most of it, and of two alongside as much, the
    /// first.
    pub fn neighbour(&self, from: Pane, toward: Toward) -> Option<Pane> {
        let tiles = self.tile(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(1000.0)),
            Vec2::ZERO,
        );
        let (_, here) = tiles.panes.iter().find(|(p, _)| *p == from)?;
        // Two panes that meet have a divider between them, not a shared edge.
        let near = |a: f32, b: f32| (a - b).abs() <= SPLIT_DIVIDER + 1.0;
        tiles
            .panes
            .iter()
            .filter(|(p, _)| *p != from)
            .filter_map(|(p, r)| {
                let (meets, overlap) = match toward {
                    Toward::Left => (
                        near(r.right(), here.left()),
                        r.y_range().intersection(here.y_range()),
                    ),
                    Toward::Right => (
                        near(r.left(), here.right()),
                        r.y_range().intersection(here.y_range()),
                    ),
                    Toward::Up => (
                        near(r.bottom(), here.top()),
                        r.x_range().intersection(here.x_range()),
                    ),
                    Toward::Down => (
                        near(r.top(), here.bottom()),
                        r.x_range().intersection(here.x_range()),
                    ),
                };
                let span = overlap.max - overlap.min;
                (meets && span > 0.0).then_some((*p, span))
            })
            // Reversed, because `max_by` keeps the last of equals: two panes
            // alongside the same length of edge go to the first, top or left.
            .rev()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(p, _)| p)
    }

    /// The layout as one line of text, for the session file: a pane is its
    /// number, a split is `R` or `B`, its ratio, and its two halves in
    /// brackets - `R0.5(0,B0.5(1,2))`.
    ///
    /// Text rather than nested tables: TOML spells a tree out as a table per
    /// node, which nobody can read back, and a hand edit that breaks this is
    /// caught by [`Node::parse`] rather than by a restore half done.
    pub fn encode(&self) -> String {
        match self {
            Node::Leaf(pane) => pane.0.to_string(),
            Node::Split {
                dir,
                ratio,
                first,
                second,
            } => {
                let d = match dir {
                    SplitDir::Right => 'R',
                    SplitDir::Bottom => 'B',
                };
                format!("{d}{ratio}({},{})", first.encode(), second.encode())
            }
        }
    }

    /// Reads back what [`Node::encode`] wrote, or `None` for anything else.
    pub fn parse(text: &str) -> Option<Node> {
        let (node, rest) = parse_node(text.trim())?;
        rest.is_empty().then_some(node)
    }

    /// Whether this is a layout of exactly the panes `0..count`, each once.
    /// A session file can say anything, and a tree naming a pane that was not
    /// opened would draw nothing where it said one was.
    pub fn covers(&self, count: usize) -> bool {
        let mut panes = self.panes();
        panes.sort();
        panes.len() == count && panes.iter().enumerate().all(|(i, p)| p.0 == i)
    }
}

/// Which side of `pane` a tab dropped at `at` goes against: the edge the
/// pointer is nearest, measured in proportion to the pane's own size.
///
/// Every point of a pane is one side or another - its diagonals divide it -
/// so there is no dead middle where a drop does nothing. In proportion, so a
/// wide pane is not all left and right: a point a quarter of the way down is
/// in the top quarter of the pane whatever shape it is.
pub fn drop_side(pane: Rect, at: Pos2) -> Toward {
    let x = (at.x - pane.left()) / pane.width().max(1.0);
    let y = (at.y - pane.top()) / pane.height().max(1.0);
    let near = [
        (x, Toward::Left),
        (1.0 - x, Toward::Right),
        (y, Toward::Up),
        (1.0 - y, Toward::Down),
    ];
    near.into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or(Toward::Right, |(_, side)| side)
}

/// The part of `pane` a tab dropped against `side` will take: the half on
/// that side, which is what the split it makes starts at.
pub fn drop_preview(pane: Rect, side: Toward) -> Rect {
    let half = pane.size() / 2.0;
    match side {
        Toward::Left => Rect::from_min_size(pane.min, Vec2::new(half.x, pane.height())),
        Toward::Right => Rect::from_min_max(Pos2::new(pane.center().x, pane.top()), pane.max),
        Toward::Up => Rect::from_min_size(pane.min, Vec2::new(pane.width(), half.y)),
        Toward::Down => Rect::from_min_max(Pos2::new(pane.left(), pane.center().y), pane.max),
    }
}

/// The two halves of `room` and the divider between them, and the room they
/// share.
fn halves(room: Rect, dir: SplitDir, ratio: f32, spacing: Vec2) -> (Rect, Rect, Rect, f32) {
    match dir {
        SplitDir::Right => {
            let gap = spacing.x * 2.0 + SPLIT_DIVIDER;
            let usable = (room.width() - gap).max(0.0);
            let first = split_extent(usable, ratio);
            let a = Rect::from_min_size(room.min, Vec2::new(first, room.height()));
            let d = Rect::from_min_size(
                Pos2::new(a.right() + spacing.x, room.top()),
                Vec2::new(SPLIT_DIVIDER, room.height()),
            );
            let b = Rect::from_min_max(Pos2::new(d.right() + spacing.x, room.top()), room.max);
            (a, d, b, usable)
        }
        SplitDir::Bottom => {
            let gap = spacing.y * 2.0 + SPLIT_DIVIDER;
            let usable = (room.height() - gap).max(0.0);
            let first = split_extent(usable, ratio);
            let a = Rect::from_min_size(room.min, Vec2::new(room.width(), first));
            let d = Rect::from_min_size(
                Pos2::new(room.left(), a.bottom() + spacing.y),
                Vec2::new(room.width(), SPLIT_DIVIDER),
            );
            let b = Rect::from_min_max(Pos2::new(room.left(), d.bottom() + spacing.y), room.max);
            (a, d, b, usable)
        }
    }
}

fn parse_node(text: &str) -> Option<(Node, &str)> {
    let dir = match text.chars().next()? {
        'R' => SplitDir::Right,
        'B' => SplitDir::Bottom,
        _ => {
            let end = text
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(text.len());
            let pane = text[..end].parse().ok()?;
            return Some((Node::Leaf(Pane(pane)), &text[end..]));
        }
    };
    let rest = &text[1..];
    let open = rest.find('(')?;
    let ratio: f32 = rest[..open].parse().ok()?;
    // A NaN would survive every clamp the layout applies to it.
    let ratio = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let (first, rest) = parse_node(&rest[open + 1..])?;
    let rest = rest.strip_prefix(',')?;
    let (second, rest) = parse_node(rest)?;
    let rest = rest.strip_prefix(')')?;
    Some((
        Node::Split {
            dir,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        },
        rest,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(n: usize) -> Node {
        Node::Leaf(Pane(n))
    }

    /// 0 on the left, 1 over 2 on the right.
    fn three() -> Node {
        Node::split(
            SplitDir::Right,
            leaf(0),
            Node::split(SplitDir::Bottom, leaf(1), leaf(2)),
        )
    }

    fn room() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 600.0))
    }

    #[test]
    fn docking_against_a_side_puts_the_new_pane_on_that_side() {
        let mut tree = leaf(0);
        assert!(tree.dock(Pane(0), Toward::Left, leaf(1)));
        assert_eq!(tree.panes(), vec![Pane(1), Pane(0)]);
        assert!(tree.dock(Pane(0), Toward::Down, leaf(2)));
        assert_eq!(tree.panes(), vec![Pane(1), Pane(0), Pane(2)]);
        let tiles = tree.tile(room(), Vec2::ZERO);
        let at = |p| tiles.panes.iter().find(|(q, _)| *q == Pane(p)).unwrap().1;
        assert!(at(1).right() <= at(0).left());
        assert!(at(0).bottom() <= at(2).top());
    }

    #[test]
    fn docking_against_a_pane_that_is_not_there_changes_nothing() {
        let mut tree = three();
        assert!(!tree.dock(Pane(7), Toward::Left, leaf(3)));
        assert_eq!(tree, three());
    }

    /// The pane beside the one that goes takes its room, and the numbers
    /// close up the way the list of sessions does.
    #[test]
    fn removing_a_pane_hands_its_room_to_the_one_beside_it_and_closes_the_numbers_up() {
        let mut tree = three();
        assert_eq!(tree.remove(Pane(1)), Some(Pane(1)));
        assert_eq!(tree, Node::split(SplitDir::Right, leaf(0), leaf(1)));
    }

    #[test]
    fn removing_the_first_pane_renumbers_every_other() {
        let mut tree = three();
        assert_eq!(tree.remove(Pane(0)), Some(Pane(0)));
        assert_eq!(tree, Node::split(SplitDir::Bottom, leaf(0), leaf(1)));
    }

    #[test]
    fn the_last_pane_cannot_be_removed() {
        let mut tree = leaf(0);
        assert_eq!(tree.remove(Pane(0)), None);
        assert_eq!(tree, leaf(0));
    }

    #[test]
    fn neighbours_are_found_on_screen_rather_than_in_the_tree() {
        let tree = three();
        assert_eq!(tree.neighbour(Pane(0), Toward::Right), Some(Pane(1)));
        assert_eq!(tree.neighbour(Pane(2), Toward::Left), Some(Pane(0)));
        assert_eq!(tree.neighbour(Pane(1), Toward::Down), Some(Pane(2)));
        assert_eq!(tree.neighbour(Pane(2), Toward::Up), Some(Pane(1)));
        assert_eq!(tree.neighbour(Pane(0), Toward::Left), None);
        assert_eq!(tree.neighbour(Pane(0), Toward::Up), None);
    }

    /// Of two panes meeting an edge, the one alongside more of it.
    #[test]
    fn the_neighbour_is_the_one_alongside_most_of_the_edge() {
        let mut tree = three();
        tree.set_ratio(1, 0.8);
        assert_eq!(tree.neighbour(Pane(0), Toward::Right), Some(Pane(1)));
        tree.set_ratio(1, 0.2);
        assert_eq!(tree.neighbour(Pane(0), Toward::Right), Some(Pane(2)));
    }

    /// A pane moved against another's side lands there, numbered as it
    /// was, and the room it left closes up.
    #[test]
    fn a_relocated_pane_keeps_its_number_and_lands_where_it_was_dropped() {
        let mut tree = three();
        assert!(tree.relocate(Pane(0), Pane(2), Toward::Down));
        assert!(tree.covers(3));
        assert_eq!(tree.neighbour(Pane(2), Toward::Down), Some(Pane(0)));
        assert_eq!(tree.neighbour(Pane(1), Toward::Down), Some(Pane(2)));
        assert_eq!(
            tree.neighbour(Pane(1), Toward::Left),
            None,
            "0's room is gone"
        );
    }

    #[test]
    fn a_pane_cannot_be_relocated_onto_itself_or_a_missing_one() {
        let mut tree = three();
        assert!(!tree.relocate(Pane(1), Pane(1), Toward::Left));
        assert!(!tree.relocate(Pane(1), Pane(9), Toward::Left));
        assert_eq!(tree, three());
    }

    #[test]
    fn swapping_trades_places_and_keeps_the_shape() {
        let mut tree = three();
        tree.swap(Pane(0), Pane(2));
        assert_eq!(
            tree,
            Node::split(
                SplitDir::Right,
                leaf(2),
                Node::split(SplitDir::Bottom, leaf(1), leaf(0)),
            )
        );
    }

    #[test]
    fn flipping_turns_only_the_split_the_pane_is_in() {
        let mut tree = three();
        assert!(tree.flip_around(Pane(2)));
        assert_eq!(
            tree,
            Node::split(
                SplitDir::Right,
                leaf(0),
                Node::split(SplitDir::Right, leaf(1), leaf(2)),
            )
        );
        assert!(!leaf(0).flip_around(Pane(0)));
    }

    /// Every pane lies inside the room, no two overlap, and every divider
    /// lies between the panes it divides.
    #[test]
    fn the_tiles_fill_the_room_without_overlapping() {
        let tiles = three().tile(room(), Vec2::new(2.0, 2.0));
        assert_eq!(tiles.panes.len(), 3);
        assert_eq!(tiles.dividers.len(), 2);
        for (i, (_, a)) in tiles.panes.iter().enumerate() {
            assert!(room().contains_rect(*a));
            for (_, b) in &tiles.panes[i + 1..] {
                assert!(!a.intersects(b.shrink(0.01)), "{a:?} and {b:?}");
            }
            for d in &tiles.dividers {
                assert!(!a.intersects(d.rect.shrink(0.01)));
            }
        }
    }

    /// The ids `tile` hands out are the ones `set_ratio` answers to.
    #[test]
    fn a_divider_drag_moves_the_split_it_was_drawn_for() {
        let mut tree = three();
        let inner = tree.tile(room(), Vec2::ZERO).dividers[1];
        assert_eq!(inner.dir, SplitDir::Bottom);
        tree.set_ratio(inner.id, 0.25);
        let after = tree.tile(room(), Vec2::ZERO).dividers[1];
        assert_eq!(after.ratio, 0.25);
        assert_eq!(tree.tile(room(), Vec2::ZERO).dividers[0].ratio, 0.5);
    }

    /// However deep the splits go, no pane is squeezed below the minimum
    /// while there is room for it.
    #[test]
    fn nested_splits_keep_every_pane_at_least_the_minimum() {
        let mut tree = three();
        tree.set_ratio(0, 1.0);
        tree.set_ratio(1, 0.0);
        let tiles = tree.tile(room(), Vec2::ZERO);
        for (_, r) in tiles.panes {
            assert!(r.width() >= 79.0 && r.height() >= 79.0, "{r:?}");
        }
    }

    #[test]
    fn a_drop_goes_against_the_edge_the_pointer_is_nearest() {
        let pane = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(800.0, 200.0));
        assert_eq!(drop_side(pane, Pos2::new(120.0, 200.0)), Toward::Left);
        assert_eq!(drop_side(pane, Pos2::new(880.0, 200.0)), Toward::Right);
        // A quarter of the way down a wide pane is its top, though it is
        // nearer the left edge in points than the top in proportion.
        assert_eq!(drop_side(pane, Pos2::new(500.0, 130.0)), Toward::Up);
        assert_eq!(drop_side(pane, Pos2::new(500.0, 290.0)), Toward::Down);
    }

    #[test]
    fn the_preview_is_the_half_the_drop_will_take() {
        let pane = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 200.0));
        assert_eq!(
            drop_preview(pane, Toward::Right),
            Rect::from_min_max(Pos2::new(400.0, 0.0), Pos2::new(800.0, 200.0))
        );
        assert_eq!(
            drop_preview(pane, Toward::Up),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 100.0))
        );
    }

    #[test]
    fn a_layout_reads_back_as_it_was_written() {
        let mut tree = three();
        tree.set_ratio(0, 0.3);
        let text = tree.encode();
        assert_eq!(text, "R0.3(0,B0.5(1,2))");
        assert_eq!(Node::parse(&text), Some(tree));
    }

    #[test]
    fn a_broken_layout_is_refused_rather_than_half_read() {
        for text in [
            "",
            "R0.5(0,1",
            "R0.5(0,1)x",
            "Q0.5(0,1)",
            "R(0,1)",
            "Rx(0,1)",
        ] {
            assert_eq!(Node::parse(text), None, "{text}");
        }
        assert_eq!(
            Node::parse("RNaN(0,1)"),
            Some(Node::split(SplitDir::Right, leaf(0), leaf(1)))
        );
    }

    #[test]
    fn a_layout_must_name_each_pane_exactly_once() {
        assert!(three().covers(3));
        assert!(!three().covers(4));
        assert!(!Node::split(SplitDir::Right, leaf(0), leaf(0)).covers(2));
        assert!(!Node::split(SplitDir::Right, leaf(0), leaf(2)).covers(2));
    }
}
