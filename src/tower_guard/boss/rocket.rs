use super::{BossCycle, BossEvent, BossState, Rocket};

impl BossState {
    pub(super) fn update_rocket(&mut self, dt: f32, events: &mut Vec<BossEvent>) {
        match self.cycle {
            BossCycle::PrepareRocket if self.cycle_time >= 1.75 => {
                self.cycle = BossCycle::RocketFlying;
                self.cycle_time = 0.0;
                self.rocket = Some(Rocket {
                    x: self.x + 0.18,
                    y: 0.45,
                    progress: 0.0,
                    resolved: false,
                });
                events.push(BossEvent::LaunchRocket);
            }
            BossCycle::RocketFlying => {
                if let Some(rocket) = &mut self.rocket {
                    rocket.progress = (rocket.progress + dt / 2.0).min(1.0);
                    rocket.y = 0.43 + rocket.progress * 0.42;
                    // cycle_time advances in small frame steps; keep rocket motion independent.
                    if rocket.progress >= 1.0 && !rocket.resolved {
                        rocket.resolved = true;
                        events.push(BossEvent::RocketImpact);
                        self.cycle = BossCycle::ExposedCore;
                        self.cycle_time = 0.0;
                    }
                }
                if self.cycle == BossCycle::RocketFlying && self.cycle_time >= 3.25 {
                    self.cycle = BossCycle::ExposedCore;
                    self.cycle_time = 0.0;
                }
            }
            BossCycle::ExposedCore if self.cycle_time >= 2.45 => {
                self.cycle = BossCycle::Recover;
                self.cycle_time = 0.0;
                self.rocket = None;
            }
            BossCycle::Recover if self.cycle_time >= 0.95 => {
                self.cycle = BossCycle::PrepareRocket;
                self.cycle_time = 0.0;
                self.target_x = if self.target_x < 0.5 { 0.62 } else { 0.38 };
            }
            BossCycle::Dying if self.cycle_time >= 2.1 => {
                self.cycle = BossCycle::Dead;
                self.cycle_time = 0.0;
                self.rocket = None;
            }
            BossCycle::Dead => {}
            _ => {}
        }
    }
}
