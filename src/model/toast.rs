use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Severity displayed in a toast's title and border.
#[allow(dead_code)] // Producers will be connected in a later change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Info,
    Warning,
    Error,
}

/// How long a toast stays visible, starting with its first rendered frame.
#[allow(dead_code)] // Producers will be connected in a later change.
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

#[cfg(test)]
mod tests {
    use super::*;

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
