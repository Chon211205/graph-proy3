use super::{BossCycle, BossEvent, BossState};

impl BossState {
    pub(super) fn update_hat(&mut self, events: &mut Vec<BossEvent>) {
        let (duration, next) = match self.cycle {
            BossCycle::Reposition => (1.05, BossCycle::Covered),
            BossCycle::Covered => (0.90, BossCycle::Windup),
            BossCycle::Windup => (1.10, BossCycle::Vulnerable),
            BossCycle::Vulnerable if self.interrupted => (0.0, BossCycle::Reposition),
            BossCycle::Vulnerable => (2.45, BossCycle::Attack),
            BossCycle::Attack => (0.70, BossCycle::Reposition),
            BossCycle::Dying => (2.0, BossCycle::Dead),
            BossCycle::Dead => return,
            _ => (0.8, BossCycle::Reposition),
        };
        if self.cycle_time < duration {
            return;
        }
        match self.cycle {
            BossCycle::Vulnerable if next == BossCycle::Reposition => {
                self.interrupted = false;
            }
            BossCycle::Vulnerable => {
                self.hits_this_window = 0;
                self.target_x = if self.target_x < 0.5 { 0.64 } else { 0.36 };
            }
            BossCycle::Reposition => {
                self.target_x = if self.target_x > 0.5 { 0.39 } else { 0.61 };
            }
            BossCycle::Attack => {
                // Apply each hat attack only on the one transition into this state.
                self.attacks_resolved += 1;
                events.push(BossEvent::LoseDefense);
            }
            BossCycle::Dying => {
                self.cycle = BossCycle::Dead;
                self.cycle_time = 0.0;
                return;
            }
            _ => {}
        }
        self.cycle = next;
        self.cycle_time = 0.0;
    }
}
