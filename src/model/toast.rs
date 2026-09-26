use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::Model;

/// Severity displayed in a toast's title and border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Info,
    Warning,
    Error,
}

/// How long a toast stays visible, starting with its first rendered frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastDuration {
    Short,
    Long,
}

impl ToastDuration {
    pub const fn as_duration(self) -> Duration {
        Duration::from_secs(match self {
            Self::Short => 3,
            Self::Long => 6,
        })
    }
}

#[derive(Debug)]
pub(crate) struct Toast {
    pub message: String,
    pub kind: ToastType,
    duration: ToastDuration,
    expires_at: Option<Instant>,
}

impl Toast {
    /// Starts the lifetime only once the renderer has room for this toast.
    pub fn mark_visible(&mut self, now: Instant) {
        self.expires_at
            .get_or_insert_with(|| now + self.duration.as_duration());
    }
}

/// FIFO buffer of visible and pending toasts. Pending messages do not expire.
#[derive(Debug, Default)]
pub struct ToastBuffer {
    entries: VecDeque<Toast>,
}

impl ToastBuffer {
    /// Queues a message without replacing any existing toast.
    pub fn push(&mut self, message: impl Into<String>, kind: ToastType, duration: ToastDuration) {
        self.entries.push_back(Toast {
            message: message.into(),
            kind,
            duration,
            expires_at: None,
        });
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Toast> {
        self.entries.iter_mut()
    }

    /// Removes every expired toast, including short ones behind long ones.
    /// Returns whether the view needs to be redrawn.
    pub fn remove_expired(&mut self, now: Instant) -> bool {
        let previous_len = self.entries.len();
        self.entries
            .retain(|toast| toast.expires_at.is_none_or(|expiry| expiry > now));
        self.entries.len() != previous_len
    }

    /// Returns the next visible toast's expiry for the input loop's timeout.
    pub fn next_expiration(&self, now: Instant) -> Option<Duration> {
        self.entries
            .iter()
            .filter_map(|toast| toast.expires_at)
            .min()
            .map(|expiry| expiry.saturating_duration_since(now))
    }
}

impl Model {
    /// Shows a short informational message.
    pub fn toast_info(&mut self, message: impl Into<String>) {
        self.notify(message.into(), ToastType::Info, ToastDuration::Short);
    }

    /// Shows a long warning about a fallback or incomplete action.
    pub fn toast_warning(&mut self, message: impl Into<String>) {
        self.notify(message.into(), ToastType::Warning, ToastDuration::Long);
    }

    /// Shows a long error about a failed operation.
    pub fn toast_error(&mut self, message: impl Into<String>) {
        self.notify(message.into(), ToastType::Error, ToastDuration::Long);
    }

    /// Avoids filling the queue with the same recurring failure.
    fn notify(&mut self, message: String, kind: ToastType, duration: ToastDuration) {
        if !self.settings().toasts() {
            return;
        }
        for toast in &self.toasts.entries {
            if toast.message == message && toast.kind == kind {
                return;
            }
        }
        self.toasts.push(message, kind, duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_failures_are_deduplicated_and_severity_controls_duration() {
        let mut model = Model::new();
        model.toasts.entries.clear();
        model.toast_error("save failed");
        model.toast_error("save failed");
        model.toast_warning("using defaults");
        model.toast_info("nothing to delete");
        assert_eq!(model.toasts.entries.len(), 3);
        assert_eq!(model.toasts.entries[0].kind, ToastType::Error);
        assert_eq!(model.toasts.entries[0].duration, ToastDuration::Long);
        assert_eq!(model.toasts.entries[1].kind, ToastType::Warning);
        assert_eq!(model.toasts.entries[1].duration, ToastDuration::Long);
        assert_eq!(model.toasts.entries[2].kind, ToastType::Info);
        assert_eq!(model.toasts.entries[2].duration, ToastDuration::Short);
    }

    #[test]
    fn mixed_durations_expire_independently_at_the_deadline() {
        let now = Instant::now();
        let mut buffer = ToastBuffer::default();
        buffer.push("long", ToastType::Warning, ToastDuration::Long);
        buffer.push("short", ToastType::Info, ToastDuration::Short);
        for toast in buffer.iter_mut() {
            toast.mark_visible(now);
        }
        assert!(!buffer.remove_expired(now + Duration::from_millis(2_999)));
        assert_eq!(buffer.next_expiration(now), Some(Duration::from_secs(3)));
        assert!(buffer.remove_expired(now + Duration::from_secs(3)));
        assert_eq!(buffer.entries.len(), 1);
        assert_eq!(buffer.entries[0].message, "long");
        assert!(buffer.remove_expired(now + Duration::from_secs(6)));
        assert_eq!(buffer.next_expiration(now), None);
    }

    #[test]
    fn pending_messages_get_their_full_duration_and_redraws_do_not_restart_it() {
        let now = Instant::now();
        let mut buffer = ToastBuffer::default();
        buffer.push("pending", ToastType::Error, ToastDuration::Short);
        assert!(!buffer.remove_expired(now + Duration::from_secs(60)));
        assert_eq!(buffer.next_expiration(now), None);
        buffer.entries[0].mark_visible(now + Duration::from_secs(60));
        buffer.entries[0].mark_visible(now + Duration::from_secs(61));
        assert_eq!(
            buffer.next_expiration(now + Duration::from_secs(61)),
            Some(Duration::from_secs(2))
        );
        assert!(buffer.remove_expired(now + Duration::from_secs(63)));
    }
}
