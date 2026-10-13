//! The message in the footer, and the button some of them carry.
//!
//! A message with a button is an offer - the usage report after an update -
//! and an offer is lost if the next message simply replaces it. Something as
//! ordinary as an update check refused by the proxy arrives on the first
//! frames after starting, exactly when the offer goes up. So a message with a
//! button is set aside while another one is showing, and comes back, with its
//! time to live restarted, once that one is gone.

use std::time::{Duration, Instant};

use crate::i18n::tr;
use crate::ui::panels::UsageOffer;

/// What the button beside a footer message does.
#[derive(Clone, Debug)]
pub enum Action {
    /// Open the usage report dialog with this offer.
    ReviewUsageReport(UsageOffer),
}

impl Action {
    pub fn label(&self) -> &'static str {
        match self {
            Action::ReviewUsageReport(_) => tr("Review..."),
        }
    }
}

/// The footer: the message showing, when it went up, its button, and an
/// offer waiting for the footer to be free again.
#[derive(Default)]
pub struct Footer {
    text: Option<String>,
    at: Option<Instant>,
    action: Option<Action>,
    held: Option<(String, Action)>,
}

impl Footer {
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn action(&self) -> Option<&Action> {
        self.action.as_ref()
    }

    /// Shows `text`, setting aside an offer it would otherwise replace.
    pub fn set(&mut self, text: String) {
        if let (Some(action), Some(shown)) = (self.action.take(), self.text.take()) {
            self.held = Some((shown, action));
        }
        self.text = Some(text);
        self.at = Some(Instant::now());
    }

    /// Shows `text` with a button that does `action`.
    pub fn offer(&mut self, text: String, action: Action) {
        self.set(text);
        self.action = Some(action);
    }

    /// Takes the message down: dismissed, or its time is up. An offer set
    /// aside comes back; an offer taken down this way is declined.
    pub fn clear(&mut self) {
        self.text = None;
        self.at = None;
        if self.action.take().is_none() {
            if let Some((text, action)) = self.held.take() {
                self.offer(text, action);
            }
        }
    }

    /// The button was pressed: the message goes, and its action is handed
    /// back to be carried out.
    pub fn take_action(&mut self) -> Option<Action> {
        let action = self.action.take();
        self.text = None;
        self.at = None;
        action
    }

    /// How long the message has left before `life` is up, or `None` once it
    /// has run out or there is no message to time.
    pub fn left(&self, life: Duration) -> Option<Duration> {
        self.text.as_ref()?;
        life.checked_sub(self.at?.elapsed())
            .filter(|left| !left.is_zero())
    }

    pub fn is_shown(&self) -> bool {
        self.text.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer() -> Action {
        Action::ReviewUsageReport(UsageOffer {
            report: "report".into(),
            email: String::new(),
        })
    }

    #[test]
    fn an_offer_outlives_the_message_that_replaced_it() {
        let mut footer = Footer::default();
        footer.offer("help?".into(), offer());
        footer.set("could not check for updates".into());
        assert_eq!(footer.text(), Some("could not check for updates"));
        assert!(footer.action().is_none(), "the error has no button");
        footer.clear();
        assert_eq!(footer.text(), Some("help?"), "the offer comes back");
        assert!(footer.action().is_some());
    }

    #[test]
    fn a_dismissed_offer_does_not_come_back() {
        let mut footer = Footer::default();
        footer.offer("help?".into(), offer());
        footer.clear();
        assert!(!footer.is_shown());
        footer.set("something else".into());
        footer.clear();
        assert!(!footer.is_shown(), "declined is declined");
    }

    #[test]
    fn pressing_the_button_hands_the_action_back_and_clears_the_footer() {
        let mut footer = Footer::default();
        footer.offer("help?".into(), offer());
        assert!(matches!(
            footer.take_action(),
            Some(Action::ReviewUsageReport(_))
        ));
        assert!(!footer.is_shown());
        footer.clear();
        assert!(!footer.is_shown());
    }

    #[test]
    fn a_message_left_up_runs_out() {
        let mut footer = Footer::default();
        assert!(
            footer.left(Duration::from_secs(8)).is_none(),
            "nothing to time"
        );
        footer.set("x".into());
        assert!(footer.left(Duration::from_secs(8)).is_some());
        assert!(footer.left(Duration::ZERO).is_none());
    }
}
