//! The tabs that were open when the app last closed, for
//! [`super::Settings::remember_open_tabs`].
//!
//! A model of its own rather than serde on `Tab` or `Split`: those hold live
//! sessions, and the only part of a tab worth reopening is what it connects to
//! and what it was called. What it connects to is its [`Profile`], which holds
//! no password - that lives in the keychain - so nothing typed at a prompt, and
//! no secret, reaches this file.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::Profile;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedSession {
    /// Index into `tabs` of the tab that was in front.
    pub active: usize,
    pub tabs: Vec<SavedTab>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedTab {
    pub profile: Profile,
    // TOML has no way to write a `None`, so each of these is left out instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The colour the tab was marked with, as `#rrggbb`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// The tab's other sessions, in pane order: the first of these is pane 2.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub panes: Vec<SavedPane>,
    /// Where each pane sat, as the app's tiling writes a layout down - see
    /// `Node::encode`. Kept as text here: the config tree describes a file,
    /// and the shape of a layout is the app's business.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    /// Which pane had the keyboard, counted from 0 for the tab's own.
    #[serde(skip_serializing_if = "is_zero")]
    pub focus: usize,
    /// The one other pane a tab could have before a tab could hold several.
    /// Read, so a session saved by an older build comes back split, and never
    /// written again.
    #[serde(skip_serializing)]
    pub split: Option<SavedSplit>,
    /// Whether that pane had the keyboard. Read and never written, as `split`.
    #[serde(skip_serializing)]
    pub second_focused: bool,
    /// The namespace the pane was last seen in, read off its prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// What the pane showed when it closed, as plain text, oldest line first.
    /// Put back above the new session's output so the tab still says what was
    /// being done in it; the session behind it is a fresh one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub screen: String,
}

/// One of a split tab's other sessions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedPane {
    pub profile: Profile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub screen: String,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// How an older build wrote a split down. Only ever read.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedSplit {
    pub dir: SavedDir,
    pub ratio: f32,
    pub profile: Profile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub screen: String,
}

/// Where the second pane sat. A mirror of the layout's own direction, so the
/// config tree does not reach into the app to describe a file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SavedDir {
    #[default]
    Right,
    Bottom,
}

impl SavedSession {
    /// What `path` holds, or `None` when there is nothing usable there.
    ///
    /// A malformed file is logged and skipped rather than refused: it must not
    /// stop the app from starting, and the default profile is a fine thing to
    /// open instead.
    pub fn load(path: &Path) -> Option<SavedSession> {
        let text = std::fs::read_to_string(path).ok()?;
        match toml::from_str(&text) {
            Ok(session) => Some(session),
            Err(e) => {
                log::error!("{} is invalid ({e}); not restoring it", path.display());
                None
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("serialising the session")?;
        super::write_atomic(path, text)
    }

    /// Forgets the saved session, so a switch turned off leaves nothing behind
    /// for it to find when it is turned back on months later.
    pub fn clear(path: &Path) {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("could not remove {}: {e}", path.display());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A directory of this test's own, gone by the time it returns.
    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nit-session-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create");
        dir
    }

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.into(),
            instance: name.into(),
            ..Profile::default()
        }
    }

    #[test]
    fn a_saved_session_round_trips_with_and_without_a_split() {
        let dir = tempdir("round-trip");
        let path = dir.join("session.toml");
        let session = SavedSession {
            active: 1,
            tabs: vec![
                SavedTab {
                    profile: profile("ONE"),
                    ..SavedTab::default()
                },
                SavedTab {
                    profile: profile("TWO"),
                    title: Some("renamed".into()),
                    colour: Some("#e04f4f".into()),
                    panes: vec![
                        SavedPane {
                            profile: profile("THREE"),
                            screen: "COMP80>w 1\n1".into(),
                            namespace: Some("DESENV80".into()),
                            ..SavedPane::default()
                        },
                        SavedPane {
                            profile: profile("FOUR"),
                            title: Some("four".into()),
                            ..SavedPane::default()
                        },
                    ],
                    layout: Some("R0.3(0,B0.5(1,2))".into()),
                    focus: 2,
                    ..SavedTab::default()
                },
            ],
        };
        session.save(&path).expect("save");

        let back = SavedSession::load(&path).expect("load");
        assert_eq!(back.active, 1);
        assert_eq!(back.tabs.len(), 2);
        assert!(back.tabs[0].panes.is_empty() && back.tabs[0].title.is_none());
        assert!(back.tabs[0].colour.is_none() && back.tabs[0].layout.is_none());
        let two = &back.tabs[1];
        assert_eq!(two.title.as_deref(), Some("renamed"));
        assert_eq!(two.colour.as_deref(), Some("#e04f4f"));
        assert_eq!(two.layout.as_deref(), Some("R0.3(0,B0.5(1,2))"));
        assert_eq!(two.focus, 2);
        assert_eq!(two.panes.len(), 2);
        assert_eq!(two.panes[0].profile.instance, "THREE");
        assert_eq!(two.panes[0].screen, "COMP80>w 1\n1");
        assert_eq!(two.panes[0].namespace.as_deref(), Some("DESENV80"));
        assert_eq!(two.panes[1].title.as_deref(), Some("four"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file written before a tab could hold more than two sessions still
    /// reads, its one split as it was; and it is never written that way again.
    #[test]
    fn a_split_saved_by_an_older_build_is_read_and_not_written_back() {
        let text = r#"
            active = 0
            [[tabs]]
            second_focused = true
            [tabs.profile]
            name = "TWO"
            instance = "TWO"
            [tabs.split]
            dir = "bottom"
            ratio = 0.3
            [tabs.split.profile]
            name = "THREE"
            instance = "THREE"
        "#;
        let back: SavedSession = toml::from_str(text).expect("parse");
        let split = back.tabs[0].split.as_ref().expect("split");
        assert_eq!(split.dir, SavedDir::Bottom);
        assert_eq!(split.ratio, 0.3);
        assert_eq!(split.profile.name, "THREE");
        assert!(back.tabs[0].second_focused);
        let again = toml::to_string_pretty(&back).expect("write");
        assert!(
            !again.contains("split") && !again.contains("second_focused"),
            "{again}"
        );
    }

    #[test]
    fn a_malformed_or_missing_session_file_reads_as_nothing() {
        let dir = tempdir("malformed");
        let path = dir.join("session.toml");
        assert!(SavedSession::load(&path).is_none());
        std::fs::write(&path, "tabs = 3").expect("write");
        assert!(SavedSession::load(&path).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clearing_removes_the_file_and_tolerates_there_being_none() {
        let dir = tempdir("clear");
        let path = dir.join("session.toml");
        SavedSession::default().save(&path).expect("save");
        SavedSession::clear(&path);
        assert!(!path.exists());
        SavedSession::clear(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
