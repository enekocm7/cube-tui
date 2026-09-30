use super::Model;

const RECORD_LABELS: [&str; 6] = ["single", "mo3", "ao5", "ao12", "ao50", "ao100"];

impl Model {
    /// Called exclusively for a completed attempt, never loading, importing, or editing.
    pub fn notify_personal_bests(&mut self) {
        if !self.settings().toasts() {
            return;
        }
        let Some(latest) = self.history().last() else {
            return;
        };
        if latest.was_single_record() {
            self.toast_info(format!("New session best single: {latest}"));
        }
        for (size, label) in [3, 5, 12, 50, 100].into_iter().zip(&RECORD_LABELS[1..]) {
            if self.history().latest_average_is_record(size) {
                let latest_index = self.history().len() - 1;
                let result = match size {
                    3 => self.history().mo3_at(latest_index),
                    5 => self.history().ao5_at(latest_index),
                    12 => self.history().ao12_at(latest_index),
                    50 => self.history().ao50_at(latest_index),
                    100 => self.history().ao100_at(latest_index),
                    _ => unreachable!(),
                }
                .expect("complete record average");
                self.toast_info(format!("New session best {label}: {result}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{settings::Settings, toast::ToastBuffer};
    use crate::scramble::WcaEvent;
    use crate::widgets::history::{History, Modifier};

    fn model() -> Model {
        let mut model = Model::new();
        model.toasts = ToastBuffer::default();
        model
    }

    fn record(model: &mut Model, millis: u64, modifier: Modifier) -> Vec<String> {
        model.next_scramble();
        model.toasts = ToastBuffer::default();
        model.record_solve_with_modifier(millis, modifier);
        model
            .toasts
            .iter_mut()
            .map(|toast| toast.message.clone())
            .collect()
    }

    #[test]
    fn first_records_improvements_ties_and_penalties() {
        let mut model = model();
        assert_eq!(record(&mut model, 1, Modifier::DNF), Vec::<String>::new());
        let messages = record(&mut model, 10_000, Modifier::None);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "New session best single: 00:10.000");
        assert_eq!(
            record(&mut model, 8_000, Modifier::PlusTwo),
            Vec::<String>::new()
        );
        let messages = record(&mut model, 7_000, Modifier::PlusTwo);
        let single = messages
            .iter()
            .find(|message| message.contains("best single"))
            .unwrap();
        assert!(single.contains("00:09.000+"));
    }

    #[test]
    fn every_supported_average_establishes_once_then_improves() {
        let mut model = model();
        for count in 1..=101 {
            let messages = record(&mut model, 10_000, Modifier::None);
            for (slot, size) in [1, 3, 5, 12, 50, 100].into_iter().enumerate() {
                assert_eq!(
                    messages
                        .iter()
                        .any(|message| message.contains(&format!("best {}:", RECORD_LABELS[slot]))),
                    count == size
                );
            }
        }
        assert!(
            record(&mut model, 1_000, Modifier::None)
                .iter()
                .any(|message| message.contains("best single"))
        );
        let messages = record(&mut model, 1_000, Modifier::None);
        for label in &RECORD_LABELS[1..] {
            assert!(
                messages
                    .iter()
                    .any(|message| message.contains(&format!("best {label}:")))
            );
        }
    }

    #[test]
    fn sessions_have_independent_records() {
        let mut model = model();
        record(&mut model, 10_000, Modifier::None);
        model.add_session();
        let messages = record(&mut model, 11_000, Modifier::None);
        assert_eq!(messages[0], "New session best single: 00:11.000");
        let messages = record(&mut model, 9_000, Modifier::None);
        assert_eq!(messages[0], "New session best single: 00:09.000");
    }

    #[test]
    fn restore_edits_and_deletion_recompute_without_celebrating() {
        let mut model = model();
        let mut history = History::new();
        history.add_ms(1_000, WcaEvent::Cube3x3, "R U");
        history.add_ms(2_000, WcaEvent::Cube3x3, "R U");
        model.restore_from_history([history]);
        assert!(model.toasts.iter_mut().next().is_none());
        assert!(
            record(&mut model, 3_000, Modifier::None)
                .iter()
                .all(|message| !message.contains("best single"))
        );
        model.toasts = ToastBuffer::default();
        let mut after = model.history().times()[0].snapshot();
        after.modifier = Modifier::DNF;
        model.history_mut().edit_solve(0, after);
        model.history_mut().select_index(1);
        model.history_mut().delete_selected();
        assert!(model.toasts.iter_mut().next().is_none());
        let messages = record(&mut model, 2_500, Modifier::None);
        assert!(messages[0].contains("best single"));
        assert!(messages[0].contains("00:02.500"));
    }

    #[test]
    fn notification_preferences_and_repeated_checks_do_not_replay() {
        let mut quiet_model = model();
        quiet_model.set_settings(toml::from_str::<Settings>("[display]\ntoasts = false").unwrap());
        assert_eq!(
            record(&mut quiet_model, 1_000, Modifier::None),
            Vec::<String>::new()
        );
        quiet_model.set_settings(Settings::default());
        assert_eq!(
            record(&mut quiet_model, 1_000, Modifier::None),
            Vec::<String>::new()
        );
        let mut model = model();
        record(&mut model, 1_000, Modifier::None);
        model.notify_personal_bests();
        assert_eq!(model.toasts.iter_mut().count(), 1);
    }

    #[test]
    fn averages_apply_dnf_rules_and_do_not_cross_sessions() {
        let mut model = model();
        for _ in 0..4 {
            record(&mut model, 10_000, Modifier::None);
        }
        let messages = record(&mut model, 1, Modifier::DNF);
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("best ao5"));
        assert!(messages[0].contains("00:10.000"));
        assert_eq!(record(&mut model, 1, Modifier::DNF), Vec::<String>::new());
        model.add_session();
        let messages = record(&mut model, 10_000, Modifier::None);
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("best single"));
    }
}
