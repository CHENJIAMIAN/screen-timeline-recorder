#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRecordingSnapshot {
    pub session_id: String,
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordingGuardAction {
    Pause(String),
    Resume(String),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RecordingGuardState {
    auto_paused_session_id: Option<String>,
}

impl RecordingGuardState {
    pub fn reconcile(
        &mut self,
        desktop_available: bool,
        active_recording: Option<ActiveRecordingSnapshot>,
    ) -> Option<RecordingGuardAction> {
        if !desktop_available {
            return self.handle_blocked_desktop(active_recording);
        }

        self.handle_available_desktop(active_recording)
    }

    fn handle_blocked_desktop(
        &mut self,
        active_recording: Option<ActiveRecordingSnapshot>,
    ) -> Option<RecordingGuardAction> {
        let recording = active_recording?;

        if recording.paused {
            if self.auto_paused_session_id.as_deref() != Some(recording.session_id.as_str()) {
                self.auto_paused_session_id = None;
            }
            return None;
        }

        if self.auto_paused_session_id.as_deref() == Some(recording.session_id.as_str()) {
            return None;
        }

        self.auto_paused_session_id = Some(recording.session_id.clone());
        Some(RecordingGuardAction::Pause(recording.session_id))
    }

    fn handle_available_desktop(
        &mut self,
        active_recording: Option<ActiveRecordingSnapshot>,
    ) -> Option<RecordingGuardAction> {
        let auto_paused_session_id = self.auto_paused_session_id.clone()?;
        let Some(recording) = active_recording else {
            self.auto_paused_session_id = None;
            return None;
        };

        if recording.session_id != auto_paused_session_id {
            self.auto_paused_session_id = None;
            return None;
        }

        if !recording.paused {
            self.auto_paused_session_id = None;
            return None;
        }

        self.auto_paused_session_id = None;
        Some(RecordingGuardAction::Resume(recording.session_id))
    }
}

#[cfg(test)]
mod tests {
    use super::{ActiveRecordingSnapshot, RecordingGuardAction, RecordingGuardState};

    fn snapshot(session_id: &str, paused: bool) -> ActiveRecordingSnapshot {
        ActiveRecordingSnapshot {
            session_id: session_id.to_string(),
            paused,
        }
    }

    #[test]
    fn pauses_active_recording_when_desktop_becomes_unavailable() {
        let mut state = RecordingGuardState::default();

        assert_eq!(
            state.reconcile(false, Some(snapshot("session-1", false))),
            Some(RecordingGuardAction::Pause("session-1".to_string()))
        );
    }

    #[test]
    fn does_not_auto_resume_a_recording_that_was_already_manually_paused() {
        let mut state = RecordingGuardState::default();

        assert_eq!(state.reconcile(false, Some(snapshot("session-1", true))), None);
        assert_eq!(state.reconcile(true, Some(snapshot("session-1", true))), None);
    }

    #[test]
    fn auto_resumes_only_the_same_session_after_desktop_returns() {
        let mut state = RecordingGuardState::default();

        assert_eq!(
            state.reconcile(false, Some(snapshot("session-1", false))),
            Some(RecordingGuardAction::Pause("session-1".to_string()))
        );
        assert_eq!(state.reconcile(false, Some(snapshot("session-1", false))), None);
        assert_eq!(
            state.reconcile(true, Some(snapshot("session-1", true))),
            Some(RecordingGuardAction::Resume("session-1".to_string()))
        );
    }

    #[test]
    fn clears_auto_pause_if_the_session_was_resumed_manually_before_desktop_returns() {
        let mut state = RecordingGuardState::default();

        assert_eq!(
            state.reconcile(false, Some(snapshot("session-1", false))),
            Some(RecordingGuardAction::Pause("session-1".to_string()))
        );
        assert_eq!(state.reconcile(true, Some(snapshot("session-1", false))), None);
        assert_eq!(state.reconcile(true, Some(snapshot("session-1", true))), None);
    }

    #[test]
    fn does_not_resume_a_different_session() {
        let mut state = RecordingGuardState::default();

        assert_eq!(
            state.reconcile(false, Some(snapshot("session-1", false))),
            Some(RecordingGuardAction::Pause("session-1".to_string()))
        );
        assert_eq!(state.reconcile(true, Some(snapshot("session-2", true))), None);
    }
}
