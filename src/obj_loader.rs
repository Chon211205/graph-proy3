use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }

    pub fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }

    pub fn mul(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }

    pub fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }

    pub fn cross(self, b: Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }

    pub fn normalize(self) -> Self {
        let length = self.dot(self).sqrt();

        if length < 1e-9 {
            Self::default()
        } else {
            Self::new(self.x / length, self.y / length, self.z / length)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceShader {
    Ocean,
    Waterfall,
    Earth,
    Grass,
    Rock,
    DarkRock,
    Sand,
    Wood,
    Leaves,
    YellowBlock,
    WhiteMark,
    ArwingHull,
    ArwingMetal,
    ArwingBlue,
    ArwingGlass,
    ArwingDarkMetal,
    ArwingEmission,
    ArwingYellow,
    AstraCobalt,
    AstraDeepBlue,
    AstraRedEmission,
    CowboyYellowPaint,
    CowboyRedPaint,
    CowboyOrangeEmission,
    BuzzWhiteMetal,
    BuzzGreenMetal,
    BuzzPurpleMetal,
    BuzzRedMetal,
    BuzzEmission,
    Flat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialColor {
    Fixed([u8; 3]),
    LaserCore,
    Laser,
    TurboCore,
    Turbo,
}

pub struct Face {
    pub indices: [usize; 3],
    pub material: String,
    pub group: String,
    pub shader: SurfaceShader,
    pub color: MaterialColor,
    pub smooth: bool,
}

impl Face {
    pub fn new(indices: [usize; 3], material: impl Into<String>, group: impl Into<String>) -> Self {
        let material = material.into();
        let normalized = material.to_ascii_lowercase();
        Self {
            indices,
            material,
            group: group.into(),
            shader: classify_surface(&normalized),
            color: classify_color(&normalized),
            smooth: false,
        }
    }
}

pub struct Model {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Face>,
    pub vertex_normals: Vec<Vec3>,
    bounds_center: Vec3,
    bounds_radius: f32,
}

impl Model {
    pub fn new(vertices: Vec<Vec3>, faces: Vec<Face>) -> Self {
        let mut model = Self {
            vertices,
            faces,
            vertex_normals: Vec::new(),
            bounds_center: Vec3::default(),
            bounds_radius: 0.0,
        };
        model.recalculate_bounds();
        model.recalculate_normals();
        model
    }

    pub fn bounds(&self) -> (Vec3, f32) {
        (self.bounds_center, self.bounds_radius)
    }

    pub fn recalculate_bounds(&mut self) {
        let Some(&first) = self.vertices.first() else {
            self.bounds_center = Vec3::default();
            self.bounds_radius = 0.0;
            return;
        };
        let mut min = first;
        let mut max = first;
        for vertex in &self.vertices[1..] {
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);
            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }
        self.bounds_center = min.add(max).mul(0.5);
        self.bounds_radius = self
            .vertices
            .iter()
            .map(|vertex| {
                vertex
                    .sub(self.bounds_center)
                    .dot(vertex.sub(self.bounds_center))
            })
            .fold(0.0, f32::max)
            .sqrt();
    }

    pub fn recalculate_normals(&mut self) {
        self.vertex_normals.clear();
        self.vertex_normals.resize(self.vertices.len(), Vec3::default());
        for face in &self.faces {
            if !face.smooth {
                continue;
            }
            let [i0, i1, i2] = face.indices;
            let normal = self.vertices[i1]
                .sub(self.vertices[i0])
                .cross(self.vertices[i2].sub(self.vertices[i0]));
            for index in face.indices {
                self.vertex_normals[index] = self.vertex_normals[index].add(normal);
            }
        }
        for normal in &mut self.vertex_normals {
            *normal = normal.normalize();
        }
    }
}

fn classify_surface(name: &str) -> SurfaceShader {
    if name == "casco_blanco" {
        SurfaceShader::ArwingHull
    } else if name == "titanio" {
        SurfaceShader::ArwingMetal
    } else if name == "grafito" {
        SurfaceShader::ArwingDarkMetal
    } else if name == "azul_cobalto" {
        SurfaceShader::AstraCobalt
    } else if name == "azul_profundo" {
        SurfaceShader::AstraDeepBlue
    } else if name == "vidrio_oscuro" {
        SurfaceShader::ArwingGlass
    } else if name == "luz_cian" {
        SurfaceShader::ArwingEmission
    } else if name == "luz_roja" {
        SurfaceShader::AstraRedEmission
    } else if name == "amarillo" || name == "cowboy_yellow" || name == "cowboy_cream" {
        SurfaceShader::CowboyYellowPaint
    } else if name == "rojo" || name == "cowboy_red" || name == "cowboy_darkred" {
        SurfaceShader::CowboyRedPaint
    } else if name == "cowboy_orangeglow" {
        SurfaceShader::CowboyOrangeEmission
    } else if name == "gris_oscuro" || name == "cowboy_metal" {
        SurfaceShader::ArwingMetal
    } else if name == "cowboy_graphite" {
        SurfaceShader::ArwingDarkMetal
    } else if name == "cowboy_glass" {
        SurfaceShader::ArwingGlass
    } else if name == "buzz_white" || name == "buzz_gold" {
        SurfaceShader::BuzzWhiteMetal
    } else if name == "buzz_green" || name == "buzz_blue" {
        SurfaceShader::BuzzGreenMetal
    } else if name == "buzz_purple" {
        SurfaceShader::BuzzPurpleMetal
    } else if name == "buzz_red" {
        SurfaceShader::BuzzRedMetal
    } else if name == "buzz_emission" {
        SurfaceShader::BuzzEmission
    } else if name == "buzz_glass" {
        SurfaceShader::ArwingGlass
    } else if name == "buzz_dark" {
        SurfaceShader::ArwingDarkMetal
    } else if name == "hull_white" {
        SurfaceShader::ArwingHull
    } else if name == "wing_metal" {
        SurfaceShader::ArwingMetal
    } else if name == "fin_blue" {
        SurfaceShader::ArwingBlue
    } else if name == "cockpit_blue" {
        SurfaceShader::ArwingGlass
    } else if name == "dark_metal" {
        SurfaceShader::ArwingDarkMetal
    } else if name == "engine_cyan" {
        SurfaceShader::ArwingEmission
    } else if name == "warning_yellow" {
        SurfaceShader::ArwingYellow
    } else if name.contains("oceano") {
        SurfaceShader::Ocean
    } else if name.contains("cascada") {
        SurfaceShader::Waterfall
    } else if name.contains("roca_oscura") || name.contains("deep_cave_rock") {
        SurfaceShader::DarkRock
    } else if name.contains("roca")
        || name.contains("underwater_rock")
        || name.contains("blue_stone")
    {
        SurfaceShader::Rock
    } else if name.contains("tierra") {
        SurfaceShader::Earth
    } else if name.contains("cesped") || name.contains("seaweed") {
        SurfaceShader::Grass
    } else if name.contains("arena") || name.contains("underwater_sand") {
        SurfaceShader::Sand
    } else if name.contains("tronco") {
        SurfaceShader::Wood
    } else if name.contains("hojas") {
        SurfaceShader::Leaves
    } else if name.contains("bloque_amarillo") {
        SurfaceShader::YellowBlock
    } else if name.contains("interrogacion_blanca") {
        SurfaceShader::WhiteMark
    } else {
        SurfaceShader::Flat
    }
}

fn classify_color(name: &str) -> MaterialColor {
    let fixed = if name.starts_with("mcqueen_") {
        match name {
            "mcqueen_red" => [225, 26, 35],
            "mcqueen_darkred" => [151, 14, 24],
            "mcqueen_yellow" => [255, 211, 36],
            "mcqueen_orange" => [255, 116, 20],
            "mcqueen_white" => [247, 249, 252],
            "mcqueen_glass" => [73, 156, 211],
            "mcqueen_blue" => [32, 105, 220],
            "mcqueen_dark" => [26, 29, 38],
            _ => [225, 26, 35],
        }
    } else if name.starts_with("buzz_") {
        match name {
            "buzz_white" => [238, 242, 248],
            "buzz_green" => [153, 226, 38],
            "buzz_purple" => [116, 61, 177],
            "buzz_red" => [235, 46, 60],
            "buzz_glass" => [80, 177, 200],
            "buzz_dark" => [29, 36, 54],
            "buzz_blue" => [45, 132, 240],
            "buzz_gold" => [245, 190, 42],
            "buzz_emission" => [128, 255, 30],
            _ => [238, 242, 248],
        }
    } else if matches!(
        name,
        "hull_white"
            | "wing_metal"
            | "fin_blue"
            | "cockpit_blue"
            | "dark_metal"
            | "engine_cyan"
            | "warning_yellow"
    ) {
        match name {
            "hull_white" => [238, 243, 248],
            "wing_metal" => [91, 105, 119],
            "fin_blue" => [30, 78, 190],
            "cockpit_blue" => [15, 47, 74],
            "dark_metal" => [25, 31, 42],
            "engine_cyan" => [40, 225, 242],
            "warning_yellow" => [250, 194, 34],
            _ => unreachable!(),
        }
    } else if matches!(
        name,
        "amarillo"
            | "rojo"
            | "gris_oscuro"
            | "cowboy_yellow"
            | "cowboy_red"
            | "cowboy_darkred"
            | "cowboy_metal"
            | "cowboy_graphite"
            | "cowboy_glass"
            | "cowboy_orangeglow"
            | "cowboy_cream"
    ) {
        match name {
            "amarillo" => [238, 145, 12],
            "rojo" => [190, 22, 18],
            "gris_oscuro" => [56, 64, 76],
            "cowboy_yellow" => [244, 158, 20],
            "cowboy_red" => [205, 28, 22],
            "cowboy_darkred" => [78, 16, 14],
            "cowboy_metal" => [112, 124, 134],
            "cowboy_graphite" => [26, 31, 40],
            "cowboy_glass" => [22, 73, 94],
            "cowboy_orangeglow" => [255, 72, 12],
            "cowboy_cream" => [246, 214, 150],
            _ => unreachable!(),
        }
    } else if matches!(
        name,
        "casco_blanco"
            | "azul_cobalto"
            | "azul_profundo"
            | "vidrio_oscuro"
            | "luz_cian"
            | "luz_roja"
            | "titanio"
            | "grafito"
    ) {
        match name {
            "casco_blanco" => [238, 244, 250],
            "azul_cobalto" => [28, 92, 224],
            "azul_profundo" => [14, 38, 105],
            "vidrio_oscuro" => [22, 70, 94],
            "luz_cian" => [40, 205, 255],
            "luz_roja" => [255, 35, 42],
            "titanio" => [104, 124, 148],
            "grafito" => [28, 35, 48],
            _ => unreachable!(),
        }
    } else if name.contains("cyan_lights") {
        [40, 205, 255]
    } else if name.contains("kirby_white_highlights") {
        [250, 252, 255]
    } else if name.contains("kirby_blue_highlights") {
        [72, 142, 242]
    } else if name.contains("kirby_burgundy_eyes") {
        [92, 22, 48]
    } else if name.contains("kirby_dark_eyes") {
        [35, 22, 48]
    } else if name.contains("kirby_red_feet") {
        [220, 42, 72]
    } else if name.contains("mechanical_dark") {
        [28, 35, 50]
    } else if name.contains("engine_metal") {
        [92, 112, 132]
    } else if name.contains("submarine_glass") {
        [45, 132, 168]
    } else if name.contains("ufo_gold_highlights") {
        [255, 218, 92]
    } else if name.contains("ufo_gold_shadow") {
        [112, 76, 26]
    } else if name.contains("ufo_metallic_gold") {
        [196, 146, 42]
    } else if name.contains("cyan_coral") || name.contains("crystal_cyan") {
        [38, 220, 226]
    } else if name.contains("orange_coral") {
        [244, 105, 42]
    } else if name.contains("pink_coral") {
        [242, 89, 164]
    } else if name.contains("purple_coral") {
        [144, 73, 205]
    } else if name.contains("bright_seaweed") {
        [50, 184, 103]
    } else if name.contains("seaweed_green") {
        [20, 108, 72]
    } else if name.contains("spike_orange") {
        [239, 113, 38]
    } else if name.contains("spike_yellow") || name.contains("yellow_glow") {
        [250, 214, 54]
    } else if name.contains("shark_blue") {
        [42, 104, 150]
    } else if name.contains("shark_light") {
        [138, 191, 203]
    } else if name.contains("jelly_purple") {
        [125, 58, 188]
    } else if name.contains("jelly_pink") {
        [241, 91, 178]
    } else if name.contains("cyan_glow")
        || name.contains("engine_cyan")
        || name.contains("cyan_engine")
    {
        [40, 225, 242]
    } else if name.contains("red_glow") {
        [255, 42, 46]
    } else if name.contains("laser_core") {
        return MaterialColor::LaserCore;
    } else if name.contains("bubble_submarine") {
        [155, 232, 255]
    } else if name.contains("laser") {
        return MaterialColor::Laser;
    } else if name.contains("turbo_core") {
        return MaterialColor::TurboCore;
    } else if name.contains("turbo") {
        return MaterialColor::Turbo;
    } else if name.contains("luz_cian") {
        [40, 205, 255]
    } else if name.contains("rosa_cuerpo") {
        [228, 58, 120]
    } else if name.contains("fucsia") {
        [235, 8, 116]
    } else if name.contains("magenta_sombra") {
        [108, 8, 59]
    } else if name.contains("coral_claro") {
        [242, 157, 137]
    } else if name.contains("durazno") {
        [255, 183, 156]
    } else if name.contains("crema") {
        [241, 221, 183]
    } else if name.contains("azul_profundo") {
        [18, 44, 98]
    } else if name.contains("azul_cobalto") {
        [34, 106, 190]
    } else if name.contains("kirby_smooth_pink") {
        [255, 158, 198]
    } else if name.contains("kirby_pink_highlights") {
        [255, 194, 218]
    } else if name.contains("kirby_blue_eye_gradient") {
        [42, 105, 215]
    } else if name.contains("kirby_light_blue_iris") {
        [100, 205, 245]
    } else if name.contains("eye_white_highlights") {
        [250, 252, 255]
    } else if name.contains("dark_gold") {
        [112, 76, 26]
    } else if name.contains("polished_gold_highlights") {
        [255, 218, 92]
    } else if name.contains("submarine_metallic_gold") {
        [196, 146, 42]
    } else if name.contains("soft_pink") {
        [255, 158, 198]
    } else if name.contains("light_pink") || name.contains("rosy_cheeks") {
        [255, 105, 158]
    } else if name.contains("burgundy") {
        [92, 22, 48]
    } else if name.contains("golden") || name.contains("gold") {
        [236, 176, 42]
    } else if name.contains("coral_rojo") {
        [238, 42, 58]
    } else if name.contains("coral_morado") {
        [151, 46, 205]
    } else if name.contains("coral_amarillo") {
        [245, 174, 42]
    } else if name.contains("planta_marina") {
        [18, 116, 76]
    } else if name.contains("ruina") {
        [84, 112, 108]
    } else if name.contains("energy_submarina") {
        [30, 220, 240]
    } else if name.contains("sun") {
        [255, 174, 48]
    } else if name.contains("tierra") {
        [112, 72, 42]
    } else if name.contains("arena") {
        [218, 188, 112]
    } else if name.contains("cesped") {
        [55, 142, 72]
    } else if name.contains("planet_land") {
        [62, 153, 91]
    } else if name.contains("planet_ice") {
        [180, 225, 238]
    } else if name.contains("planet") || name.contains("blue") {
        [47, 87, 210]
    } else if name.contains("glass") || name.contains("vidrio_oscuro") {
        [24, 60, 78]
    } else if name.contains("dark") || name.contains("gris_oscuro") {
        [43, 51, 64]
    } else if name.contains("yellow") || name.contains("amarillo") {
        [242, 197, 57]
    } else if name.contains("red") || name.contains("rojo") {
        [199, 49, 57]
    } else if name.contains("metal") {
        [103, 121, 143]
    } else {
        [220, 230, 238]
    };
    MaterialColor::Fixed(fixed)
}

fn read_index(token: &str, count: usize) -> Result<usize, String> {
    let raw = token.split('/').next().unwrap_or("");

    let index = raw
        .parse::<isize>()
        .map_err(|_| format!("Indice OBJ invalido: {token}"))?;

    let zero_based = if index > 0 {
        index - 1
    } else {
        count as isize + index
    };

    if index == 0 || zero_based < 0 || zero_based >= count as isize {
        return Err(format!("Indice fuera de rango: {token}"));
    }

    Ok(zero_based as usize)
}

pub fn load_obj(path: impl AsRef<Path>) -> Result<Model, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;

    let mut model = Model::new(Vec::new(), Vec::new());

    let mut material = "Hull White".to_string();
    let mut group = "default".to_string();
    let mut smooth = false;

    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let mut parts = line.split_whitespace();

        match parts.next() {
            Some("v") => {
                let nums: Vec<f32> = parts
                    .take(3)
                    .map(|s| s.parse::<f32>())
                    .collect::<Result<_, _>>()
                    .map_err(|e| format!("Linea {}: {e}", line_no + 1))?;

                if nums.len() == 3 {
                    model.vertices.push(Vec3::new(nums[0], nums[1], nums[2]));
                }
            }

            Some("usemtl") => {
                material = parts.collect::<Vec<_>>().join(" ");
            }

            Some("s") => {
                smooth = !matches!(parts.next(), Some("0") | Some("off") | None);
            }

            Some("g") | Some("o") => {
                group = parts.collect::<Vec<_>>().join(" ");
            }

            Some("f") => {
                let ids = parts
                    .map(|t| read_index(t, model.vertices.len()))
                    .collect::<Result<Vec<_>, _>>()?;

                for i in 1..ids.len().saturating_sub(1) {
                    let mut face = Face::new(
                        [ids[0], ids[i], ids[i + 1]],
                        material.clone(),
                        group.clone(),
                    );
                    face.smooth = smooth;
                    model.faces.push(face);
                }
            }

            _ => {}
        }
    }

    if model.vertices.is_empty() || model.faces.is_empty() {
        return Err("OBJ sin vertices o caras".into());
    }

    model.recalculate_bounds();
    model.recalculate_normals();

    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_caches_fixed_material_classification() {
        let face = Face::new([0, 1, 2], "Underwater_Rock", "terrain");
        assert_eq!(face.shader, SurfaceShader::Rock);
        assert_eq!(face.color, MaterialColor::Fixed([220, 230, 238]));
    }

    #[test]
    fn face_preserves_dynamic_effect_materials() {
        let laser = Face::new([0, 1, 2], "laser_core", "effect");
        let turbo = Face::new([0, 1, 2], "turbo_glow", "effect");
        assert_eq!(laser.color, MaterialColor::LaserCore);
        assert_eq!(turbo.color, MaterialColor::Turbo);
    }

    #[test]
    fn arwing_materials_have_dedicated_colors_and_shaders() {
        let cases = [
            ("hull_white", [238, 243, 248], SurfaceShader::ArwingHull),
            ("wing_metal", [91, 105, 119], SurfaceShader::ArwingMetal),
            ("fin_blue", [30, 78, 190], SurfaceShader::ArwingBlue),
            ("cockpit_blue", [15, 47, 74], SurfaceShader::ArwingGlass),
            ("dark_metal", [25, 31, 42], SurfaceShader::ArwingDarkMetal),
            ("engine_cyan", [40, 225, 242], SurfaceShader::ArwingEmission),
            ("warning_yellow", [250, 194, 34], SurfaceShader::ArwingYellow),
        ];

        for (name, color, shader) in cases {
            let face = Face::new([0, 1, 2], name, "arwing");
            assert_eq!(face.color, MaterialColor::Fixed(color));
            assert_eq!(face.shader, shader);
        }
    }

    #[test]
    fn kirby_vehicle_palette_covers_space_and_submarine_materials() {
        let cases = [
            ("Cyan_Lights.006", [40, 205, 255]),
            ("Kirby_White_Highlights.005", [250, 252, 255]),
            ("Kirby_Blue_Highlights.005", [72, 142, 242]),
            ("Kirby_Burgundy_Eyes.005", [92, 22, 48]),
            ("Kirby_Dark_Eyes", [35, 22, 48]),
            ("Kirby_Red_Feet", [220, 42, 72]),
            ("Mechanical_Dark.003", [28, 35, 50]),
            ("Engine_Metal.006", [92, 112, 132]),
            ("Submarine_Glass.005", [45, 132, 168]),
            ("UFO_Gold_Highlights.005", [255, 218, 92]),
            ("UFO_Gold_Shadow.005", [112, 76, 26]),
            ("UFO_Metallic_Gold.005", [196, 146, 42]),
        ];

        for (name, color) in cases {
            assert_eq!(
                Face::new([0, 1, 2], name, "kirby").color,
                MaterialColor::Fixed(color),
                "unexpected color for {name}"
            );
        }
    }

    #[test]
    fn astra_materials_have_dedicated_colors_and_shaders() {
        let cases = [
            ("Casco_blanco", [238, 244, 250], SurfaceShader::ArwingHull),
            ("Azul_cobalto", [28, 92, 224], SurfaceShader::AstraCobalt),
            ("Azul_profundo", [14, 38, 105], SurfaceShader::AstraDeepBlue),
            ("Vidrio_oscuro", [22, 70, 94], SurfaceShader::ArwingGlass),
            ("Luz_cian", [40, 205, 255], SurfaceShader::ArwingEmission),
            ("Luz_roja", [255, 35, 42], SurfaceShader::AstraRedEmission),
            ("Titanio", [104, 124, 148], SurfaceShader::ArwingMetal),
            ("Grafito", [28, 35, 48], SurfaceShader::ArwingDarkMetal),
        ];
        for (name, color, shader) in cases {
            let face = Face::new([0, 1, 2], name, "astra");
            assert_eq!(face.color, MaterialColor::Fixed(color));
            assert_eq!(face.shader, shader);
        }
    }
}
