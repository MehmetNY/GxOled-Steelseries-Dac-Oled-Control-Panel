//! Opt-in "OSD lab" (`GXOLED_OSD_LAB=1`): runs a fixed list of experiments against the base
//! station's native volume display, one experiment per physical knob turn, so a single
//! unattended session answers several questions at once.
//!
//! Background (measured 2026-10-28 on hardware, see `docs/VOLUME_OSD_DIAGNOSTICS.md`): after a
//! dial turn the firmware shows its own home screen and silently *discards* every host frame for
//! about six seconds; the host's clock returns with the first frame written after that window.
//! No amount of host-side timing changes that, so the lab tries the device commands that could.
//!
//! This module only decides *what* to try and *when*. The device I/O stays in the draw thread
//! so that the Rust process remains the single writer to the OLED. Everything here is pure and
//! unit-tested without hardware.

use std::time::{Duration, Instant};

/// A knob turn that arrives at least this long after the previous one starts a new group.
/// The native display ends about 4.5–6.2 s after the last dial report, so 10 s separates
/// two groups safely.
pub const GROUP_IDLE: Duration = Duration::from_secs(10);

/// Delay before the first pre-action when the user has not turned the knob yet.
pub const START_GRACE: Duration = Duration::from_secs(3);

/// Something sent to the device *between* groups, so that its effect is in place before the next
/// knob turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabPre {
    SonarOn,
    SonarOff,
    HomeSimple,
    HomeRestore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabStep {
    /// Normal behaviour (2.5 s hold, then frames): the control measurement.
    Base,
    /// Send `0x95` (return to SteelSeries UI) once, at the first dial report of the group.
    UiAtDial,
    /// Send `0x95` once, when the software hold ends, then draw.
    UiAtEnd,
    /// No hold. `0x95` before every accepted dial report, then frames immediately.
    UiFrame,
    /// Pre: `0x8d 01` ("Sonar present"). Measured like `Base`.
    SonarOn,
    /// Same as `Base`, with the Sonar flag still set.
    SonarOnBase,
    /// Same as `UiAtEnd`, with the Sonar flag still set.
    SonarOnUiAtEnd,
    /// Pre: `0x8d 00`. Measured like `Base`.
    SonarOff,
    /// Pre: `0x89 01` (simple home screen). Measured like `Base`.
    HomeSimple,
    /// Pre: restore the home screen type the device reported at start. Measured like `Base`.
    HomeRestore,
}

impl LabStep {
    pub fn label(self) -> &'static str {
        match self {
            LabStep::Base => "base",
            LabStep::UiAtDial => "ui-at-dial",
            LabStep::UiAtEnd => "ui-at-end",
            LabStep::UiFrame => "ui-then-frames-no-hold",
            LabStep::SonarOn => "sonar-on",
            LabStep::SonarOnBase => "sonar-on+base",
            LabStep::SonarOnUiAtEnd => "sonar-on+ui-at-end",
            LabStep::SonarOff => "sonar-off",
            LabStep::HomeSimple => "home-simple",
            LabStep::HomeRestore => "home-restore",
        }
    }

    pub fn pre(self) -> Option<LabPre> {
        match self {
            LabStep::SonarOn => Some(LabPre::SonarOn),
            LabStep::SonarOff => Some(LabPre::SonarOff),
            LabStep::HomeSimple => Some(LabPre::HomeSimple),
            LabStep::HomeRestore => Some(LabPre::HomeRestore),
            _ => None,
        }
    }

    /// Replaces the configured software hold for this group.
    pub fn hold_override(self) -> Option<Duration> {
        match self {
            LabStep::UiFrame => Some(Duration::ZERO),
            _ => None,
        }
    }

    /// `0x95` at the first dial report of the group.
    pub fn ui_at_group_start(self) -> bool {
        matches!(self, LabStep::UiAtDial)
    }

    /// `0x95` before every accepted dial report (rate limited by the caller).
    pub fn ui_at_every_dial(self) -> bool {
        matches!(self, LabStep::UiFrame)
    }

    /// `0x95` when the software hold ends.
    pub fn ui_at_hold_end(self) -> bool {
        matches!(self, LabStep::UiAtEnd | LabStep::SonarOnUiAtEnd)
    }
}

pub fn default_steps() -> Vec<LabStep> {
    vec![
        LabStep::Base,
        LabStep::UiAtDial,
        LabStep::UiAtEnd,
        LabStep::UiFrame,
        LabStep::SonarOn,
        LabStep::SonarOnBase,
        LabStep::SonarOnUiAtEnd,
        LabStep::SonarOff,
        LabStep::HomeSimple,
        LabStep::HomeRestore,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialOutcome {
    /// True for the first dial report of a group.
    pub new_group: bool,
    /// Zero-based index of the group (counts across cycles).
    pub group: usize,
    pub step_index: usize,
    pub step: LabStep,
}

pub struct OsdLab {
    steps: Vec<LabStep>,
    /// Index of the step the *next* group will run.
    next: usize,
    /// Step of the group in progress.
    current: Option<(usize, LabStep)>,
    groups_started: usize,
    last_dial: Option<Instant>,
    pre_applied_for_next: bool,
}

impl OsdLab {
    pub fn new(steps: Vec<LabStep>) -> Self {
        assert!(!steps.is_empty());
        Self {
            steps,
            next: 0,
            current: None,
            groups_started: 0,
            last_dial: None,
            pre_applied_for_next: false,
        }
    }

    pub fn current_step(&self) -> Option<LabStep> {
        self.current.map(|(_, step)| step)
    }

    /// Register an accepted dial report.
    pub fn on_dial(&mut self, now: Instant) -> DialOutcome {
        let new_group = match self.last_dial {
            None => true,
            Some(last) => now.saturating_duration_since(last) >= GROUP_IDLE,
        };
        self.last_dial = Some(now);
        if new_group {
            let index = self.next;
            self.current = Some((index, self.steps[index]));
            self.next = (self.next + 1) % self.steps.len();
            self.pre_applied_for_next = false;
            self.groups_started += 1;
        }
        let (step_index, step) = self.current.expect("a group exists after a dial report");
        DialOutcome {
            new_group,
            group: self.groups_started - 1,
            step_index,
            step,
        }
    }

    /// The pre-action to run now, if the knob has been idle long enough and the next group has
    /// one that has not been applied yet. Returns it at most once per group.
    pub fn due_pre(&mut self, now: Instant, session_age: Duration) -> Option<(usize, LabPre)> {
        if self.pre_applied_for_next {
            return None;
        }
        let idle = match self.last_dial {
            None => session_age >= START_GRACE,
            Some(last) => now.saturating_duration_since(last) >= GROUP_IDLE,
        };
        if !idle {
            return None;
        }
        let pre = self.steps[self.next].pre()?;
        self.pre_applied_for_next = true;
        Some((self.next, pre))
    }

    /// Leading stamp digit (0-9): the step of the group in progress, or the step the next knob
    /// turn will run while the knob is idle. Doubles as an on-screen "ready" cue.
    pub fn stamp_digit(&self, now: Instant) -> u32 {
        let in_group = self
            .last_dial
            .is_some_and(|last| now.saturating_duration_since(last) < GROUP_IDLE);
        let index = if in_group {
            self.current.map(|(i, _)| i).unwrap_or(self.next)
        } else {
            self.next
        };
        (index % 10) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, secs: f32) -> Instant {
        base + Duration::from_secs_f32(secs)
    }

    #[test]
    fn dial_reports_within_ten_seconds_belong_to_one_group() {
        let t0 = Instant::now();
        let mut lab = OsdLab::new(default_steps());
        let a = lab.on_dial(t0);
        assert!(a.new_group);
        assert_eq!((a.group, a.step), (0, LabStep::Base));
        let b = lab.on_dial(at(t0, 1.4));
        assert!(!b.new_group);
        assert_eq!(b.group, 0);
        let c = lab.on_dial(at(t0, 9.0)); // still within 10 s of the previous report
        assert!(!c.new_group);
        let d = lab.on_dial(at(t0, 19.5)); // 10.5 s after the previous one
        assert!(d.new_group);
        assert_eq!((d.group, d.step), (1, LabStep::UiAtDial));
    }

    #[test]
    fn steps_cycle_after_the_last_one() {
        let t0 = Instant::now();
        let steps = default_steps();
        let n = steps.len();
        let mut lab = OsdLab::new(steps.clone());
        let mut now = t0;
        for i in 0..(n + 2) {
            let outcome = lab.on_dial(now);
            assert_eq!(outcome.step, steps[i % n]);
            now += Duration::from_secs(11);
        }
    }

    #[test]
    fn pre_actions_run_once_while_idle_and_before_their_group() {
        let t0 = Instant::now();
        let steps = vec![LabStep::Base, LabStep::SonarOn, LabStep::Base];
        let mut lab = OsdLab::new(steps);
        // Group 0 has no pre-action; its successor (SonarOn) is queued only after the knob is idle.
        lab.on_dial(t0);
        assert!(lab.due_pre(at(t0, 5.0), Duration::from_secs(5)).is_none());
        assert_eq!(
            lab.due_pre(at(t0, 10.5), Duration::from_secs(10)),
            Some((1, LabPre::SonarOn))
        );
        // Not repeated.
        assert!(lab.due_pre(at(t0, 11.0), Duration::from_secs(11)).is_none());
        // The knob turn after it starts the SonarOn group.
        let outcome = lab.on_dial(at(t0, 12.0));
        assert_eq!(outcome.step, LabStep::SonarOn);
    }

    #[test]
    fn first_step_with_a_pre_action_waits_for_the_start_grace_period() {
        let t0 = Instant::now();
        let mut lab = OsdLab::new(vec![LabStep::HomeSimple, LabStep::Base]);
        assert!(lab.due_pre(t0, Duration::from_secs(1)).is_none());
        assert_eq!(
            lab.due_pre(t0, Duration::from_secs(3)),
            Some((0, LabPre::HomeSimple))
        );
    }

    #[test]
    fn stamp_digit_announces_the_next_step_while_idle() {
        let t0 = Instant::now();
        let mut lab = OsdLab::new(default_steps());
        assert_eq!(lab.stamp_digit(t0), 0);
        lab.on_dial(t0);
        assert_eq!(lab.stamp_digit(at(t0, 3.0)), 0); // group in progress: step 0
        assert_eq!(lab.stamp_digit(at(t0, 11.0)), 1); // idle: next step is 1
        lab.on_dial(at(t0, 12.0));
        assert_eq!(lab.stamp_digit(at(t0, 13.0)), 1);
    }

    #[test]
    fn step_properties_match_their_descriptions() {
        assert_eq!(LabStep::UiFrame.hold_override(), Some(Duration::ZERO));
        assert_eq!(LabStep::Base.hold_override(), None);
        assert!(LabStep::UiAtDial.ui_at_group_start());
        assert!(LabStep::UiAtEnd.ui_at_hold_end());
        assert!(LabStep::SonarOnUiAtEnd.ui_at_hold_end());
        assert!(LabStep::UiFrame.ui_at_every_dial());
        assert_eq!(LabStep::SonarOff.pre(), Some(LabPre::SonarOff));
        assert_eq!(LabStep::Base.pre(), None);
        assert!(default_steps().len() <= 10, "the stamp shows the step as one digit");
    }
}
