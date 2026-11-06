use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    WhiteShirt,
    GreenBrute,
    BlueVest,
    OrangeClaw,
    BlueClaw,
    BlackCreature,
    HatBoss,
    RocketBoss,
}

impl EnemyKind {
    pub const ALL: [Self; 8] = [
        Self::WhiteShirt,
        Self::GreenBrute,
        Self::BlueVest,
        Self::OrangeClaw,
        Self::BlueClaw,
        Self::BlackCreature,
        Self::HatBoss,
        Self::RocketBoss,
    ];

    fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "white_shirt" => Self::WhiteShirt,
            "green_brute" => Self::GreenBrute,
            "blue_vest" => Self::BlueVest,
            "orange_claw" => Self::OrangeClaw,
            "blue_claw" => Self::BlueClaw,
            "black_creature" => Self::BlackCreature,
            "hat_boss" => Self::HatBoss,
            "rocket_boss" => Self::RocketBoss,
            _ => return None,
        })
    }

    pub fn is_boss(self) -> bool {
        matches!(self, Self::HatBoss | Self::RocketBoss)
    }

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|kind| *kind == self)
            .expect("all enemy types have a stable index")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HitBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Debug)]
pub struct CharacterDef {
    pub kind: EnemyKind,
    pub name: String,
    pub sprite_path: String,
    pub role: String,
    pub hit_points: u8,
    pub head: HitBox,
    pub weak_point: Option<HitBox>,
}

fn parse_box(value: &str) -> Result<Option<HitBox>, String> {
    if value == "-" {
        return Ok(None);
    }
    let nums = value
        .split(',')
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Invalid hit box {value}: {error}"))?;
    if nums.len() != 4 || nums.iter().any(|n| !(0.0..=1.0).contains(n)) {
        return Err(format!("Hit box must have four normalized values: {value}"));
    }
    Ok(Some(HitBox {
        left: nums[0],
        top: nums[1],
        right: nums[2],
        bottom: nums[3],
    }))
}

pub fn load_manifest() -> Result<Vec<CharacterDef>, String> {
    let manifest = include_str!("../../assets/minigames/tower_guardian/enemy_manifest.tsv");
    let mut definitions = Vec::new();
    for line in manifest
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields = line.split('|').collect::<Vec<_>>();
        if fields.len() != 8 {
            return Err(format!("Invalid tower character manifest line: {line}"));
        }
        let kind = EnemyKind::from_id(fields[0])
            .ok_or_else(|| format!("Unknown tower character id: {}", fields[0]))?;
        let hit_points = fields[5]
            .parse::<u8>()
            .map_err(|error| format!("Invalid health for {}: {error}", fields[0]))?;
        let head = parse_box(fields[6])?.ok_or_else(|| format!("Missing head box: {line}"))?;
        let weak_point = parse_box(fields[7])?;
        let sprite_path = fields[3].to_string();
        if !Path::new(&sprite_path).is_file() {
            return Err(format!("Missing prepared tower sprite: {sprite_path}"));
        }
        definitions.push(CharacterDef {
            kind,
            name: fields[1].to_string(),
            sprite_path,
            role: fields[4].to_string(),
            hit_points,
            head,
            weak_point,
        });
    }
    if definitions.len() != EnemyKind::ALL.len() {
        return Err(format!(
            "Expected eight characters, found {}",
            definitions.len()
        ));
    }
    let mut seen = [false; 8];
    for definition in &definitions {
        let index = definition.kind.index();
        if seen[index] {
            return Err(format!("Duplicate tower character: {}", definition.name));
        }
        seen[index] = true;
        if definition.name.is_empty() || definition.role.is_empty() || definition.hit_points == 0 {
            return Err(format!(
                "Incomplete tower character definition: {}",
                definition.name
            ));
        }
        if definition.kind.is_boss() != (definition.role == "boss") {
            return Err(format!(
                "Role does not match enemy type: {}",
                definition.name
            ));
        }
    }
    Ok(definitions)
}
