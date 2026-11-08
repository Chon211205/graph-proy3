use super::config::EnemyKind;

pub const NORMAL_GROUP_AMMO: u8 = 3;
pub const GROUP_WARNING_SECONDS: f32 = 0.72;
pub const GROUP_RESOLVE_SECONDS: f32 = 0.82;
pub const BETWEEN_GROUP_SECONDS: f32 = 0.68;

const ROUND_ONE: &[&[EnemyKind]] = &[
    &[EnemyKind::WhiteShirt],
    &[EnemyKind::BlueVest],
    &[EnemyKind::BlackCreature],
    &[EnemyKind::GreenBrute],
];
const ROUND_TWO: &[&[EnemyKind]] = &[
    &[EnemyKind::OrangeClaw, EnemyKind::BlueVest],
    &[EnemyKind::BlueClaw, EnemyKind::WhiteShirt],
    &[EnemyKind::GreenBrute],
    &[EnemyKind::BlackCreature, EnemyKind::OrangeClaw],
    &[EnemyKind::BlueVest, EnemyKind::WhiteShirt],
];
const ROUND_FOUR: &[&[EnemyKind]] = &[
    &[EnemyKind::GreenBrute],
    &[EnemyKind::BlueClaw, EnemyKind::WhiteShirt],
    &[EnemyKind::OrangeClaw, EnemyKind::BlackCreature],
    &[EnemyKind::GreenBrute],
    &[EnemyKind::BlueVest, EnemyKind::WhiteShirt],
    &[EnemyKind::OrangeClaw, EnemyKind::WhiteShirt],
];

pub fn group(round: u8, index: usize) -> Option<&'static [EnemyKind]> {
    let groups = match round {
        1 => ROUND_ONE,
        2 => ROUND_TWO,
        4 => ROUND_FOUR,
        _ => return None,
    };
    groups.get(index).copied()
}

pub fn group_count(round: u8) -> usize {
    match round {
        1 => ROUND_ONE.len(),
        2 => ROUND_TWO.len(),
        4 => ROUND_FOUR.len(),
        _ => 0,
    }
}

pub fn normal_lifetime(kind: EnemyKind) -> f32 {
    match kind {
        EnemyKind::OrangeClaw | EnemyKind::BlueClaw => 6.0,
        EnemyKind::GreenBrute => 9.0,
        _ => 7.7,
    }
}

pub fn round_intro(round: u8) -> f32 {
    if round == 1 {
        3.0
    } else {
        1.75
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_normal_group_fits_its_three_shot_budget() {
        for round in [1, 2, 4] {
            for index in 0..group_count(round) {
                let enemies = group(round, index).unwrap();
                let required: u8 = enemies
                    .iter()
                    .map(|kind| match kind {
                        EnemyKind::GreenBrute => 3,
                        EnemyKind::BlueVest | EnemyKind::BlueClaw | EnemyKind::BlackCreature => 2,
                        _ => 1,
                    })
                    .sum();
                assert!(enemies.len() <= 2);
                assert!(required <= NORMAL_GROUP_AMMO);
            }
        }
    }

    #[test]
    fn bosses_occupy_their_defined_rounds() {
        assert_eq!(group_count(3), 0);
        assert_eq!(group_count(5), 0);
        assert_eq!(round_intro(1), 3.0);
    }
}
