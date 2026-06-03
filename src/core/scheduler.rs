use std::time::{Duration, Instant};
use winit::event_loop::ControlFlow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerState {
    Idle,
    Dirty,
    Animating,
    WaitingUntil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedrawReason {
    Input,
    Resize,
    Animation,
    WidgetRequest,
    Command,
    FirstFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    pub interaction_linger: Duration,
    pub blink_period: Duration,
    pub idle_after_redraw: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interaction_linger: Duration::from_millis(150),
            blink_period: Duration::from_millis(500),
            idle_after_redraw: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSchedulerStats {
    pub requested_redraws: u64,
    pub immediate_redraws: u64,
    pub wait_transitions: u64,
    pub wait_until_transitions: u64,
    pub poll_transitions: u64,
    pub idle_transitions: u64,
}

#[derive(Debug, Clone)]
pub struct FrameScheduler {
    config: SchedulerConfig,
    state: SchedulerState,
    dirty: bool,
    first_frame: bool,
    last_interaction: Instant,
    next_deadline: Option<Instant>,
    repaint_interval: Option<Duration>,
    stats: FrameSchedulerStats,
}

impl FrameScheduler {
    pub fn new(now: Instant) -> Self {
        Self::with_config(now, SchedulerConfig::default())
    }

    pub fn with_config(now: Instant, config: SchedulerConfig) -> Self {
        Self {
            config,
            state: SchedulerState::Dirty,
            dirty: true,
            first_frame: true,
            last_interaction: now,
            next_deadline: Some(now),
            repaint_interval: None,
            stats: FrameSchedulerStats {
                requested_redraws: 1,
                immediate_redraws: 1,
                wait_transitions: 0,
                wait_until_transitions: 0,
                poll_transitions: 0,
                idle_transitions: 0,
            },
        }
    }

    #[inline]
    pub fn state(&self) -> SchedulerState {
        self.state
    }

    #[inline]
    pub fn stats(&self) -> &FrameSchedulerStats {
        &self.stats
    }

    #[inline]
    pub fn mark_dirty(&mut self, reason: RedrawReason, now: Instant) {
        self.dirty = true;
        self.state = SchedulerState::Dirty;
        self.next_deadline = Some(now);
        self.stats.requested_redraws += 1;
        if matches!(
            reason,
            RedrawReason::Input | RedrawReason::Resize | RedrawReason::Command
        ) {
            self.last_interaction = now;
        }
    }

    #[inline]
    pub fn set_continuous(&mut self, continuous: bool, now: Instant) {
        self.set_repaint_interval(continuous.then_some(self.config.blink_period), now);
    }

    #[inline]
    pub fn set_repaint_interval(&mut self, interval: Option<Duration>, now: Instant) {
        self.repaint_interval = interval;
        let Some(interval) = interval else {
            return;
        };

        let desired = now + interval;
        if self.next_deadline.is_none_or(|deadline| deadline > desired) {
            self.next_deadline = Some(desired);
        }
        if !self.dirty && !self.first_frame {
            self.state = SchedulerState::WaitingUntil;
        }
    }

    #[inline]
    pub fn wants_redraw(&self, now: Instant) -> bool {
        self.first_frame || self.dirty || self.next_deadline.is_some_and(|deadline| now >= deadline)
    }

    pub fn after_redraw(&mut self, now: Instant, still_continuous: bool) {
        self.after_redraw_with_interval(now, still_continuous.then_some(self.config.blink_period));
    }

    pub fn after_redraw_with_interval(&mut self, now: Instant, repaint_interval: Option<Duration>) {
        self.first_frame = false;
        self.dirty = false;
        self.repaint_interval = repaint_interval;

        if let Some(interval) = repaint_interval {
            self.state = SchedulerState::WaitingUntil;
            self.next_deadline = Some(now + interval);
        } else if now.duration_since(self.last_interaction) < self.config.interaction_linger {
            self.state = SchedulerState::WaitingUntil;
            self.next_deadline = Some(self.last_interaction + self.config.interaction_linger);
        } else {
            self.state = SchedulerState::Idle;
            self.next_deadline = None;
        }
    }

    pub fn control_flow(&mut self, now: Instant, mouse_active: bool) -> ControlFlow {
        let deadline_due = self.next_deadline.is_some_and(|deadline| now >= deadline);

        if self.first_frame || self.dirty || mouse_active || deadline_due {
            self.state = if mouse_active {
                SchedulerState::Animating
            } else {
                SchedulerState::Dirty
            };
            self.stats.poll_transitions += 1;
            return ControlFlow::Poll;
        }

        if let Some(interval) = self.repaint_interval {
            let deadline = self.next_deadline.unwrap_or(now + interval);
            self.state = SchedulerState::WaitingUntil;
            self.stats.wait_until_transitions += 1;
            return ControlFlow::WaitUntil(deadline);
        }

        if now.duration_since(self.last_interaction) < self.config.interaction_linger {
            let deadline = self.last_interaction + self.config.interaction_linger;
            self.next_deadline = Some(deadline);
            self.state = SchedulerState::WaitingUntil;
            self.stats.wait_until_transitions += 1;
            return ControlFlow::WaitUntil(deadline);
        }

        self.state = SchedulerState::Idle;
        self.next_deadline = None;
        self.stats.wait_transitions += 1;
        self.stats.idle_transitions += 1;
        ControlFlow::Wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_enters_wait_after_clean_redraw() {
        let now = Instant::now();
        let mut scheduler = FrameScheduler::new(now);
        scheduler.after_redraw(now + Duration::from_secs(1), false);
        assert_eq!(scheduler.state(), SchedulerState::Idle);
        assert!(!scheduler.wants_redraw(now + Duration::from_secs(1)));
    }

    #[test]
    fn scheduler_keeps_deadline_for_continuous_widget() {
        let now = Instant::now();
        let mut scheduler = FrameScheduler::new(now);
        scheduler.after_redraw(now, true);
        assert_eq!(scheduler.state(), SchedulerState::WaitingUntil);
        assert!(!scheduler.wants_redraw(now + Duration::from_millis(100)));
        assert!(scheduler.wants_redraw(now + Duration::from_millis(600)));
    }

    #[test]
    fn setting_repaint_interval_does_not_push_due_deadline_forward() {
        let now = Instant::now();
        let mut scheduler = FrameScheduler::new(now);
        scheduler.after_redraw_with_interval(now, Some(Duration::from_millis(16)));
        let due = now + Duration::from_millis(17);

        scheduler.set_repaint_interval(Some(Duration::from_millis(16)), due);

        assert!(scheduler.wants_redraw(due));
    }

    #[test]
    fn repaint_interval_uses_wait_until_without_polling() {
        let now = Instant::now();
        let mut scheduler = FrameScheduler::new(now);
        scheduler.after_redraw_with_interval(now, Some(Duration::from_millis(32)));

        assert!(matches!(
            scheduler.control_flow(now + Duration::from_millis(1), false),
            ControlFlow::WaitUntil(_)
        ));
    }

    #[test]
    fn due_repaint_interval_polls_for_redraw_once() {
        let now = Instant::now();
        let mut scheduler = FrameScheduler::new(now);
        scheduler.after_redraw_with_interval(now, Some(Duration::from_millis(16)));

        assert_eq!(
            scheduler.control_flow(now + Duration::from_millis(17), false),
            ControlFlow::Poll
        );
    }
}
