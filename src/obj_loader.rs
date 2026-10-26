
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
        Self::new(
            self.x - b.x,
            self.y - b.y,
            self.z - b.z,
        )
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
            Self::new(
                self.x / length,
                self.y / length,
                self.z / length,
            )
        }
    }
}

pub struct Face {
    pub indices: [usize; 3],
    pub material: String,
}

pub struct Model {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Face>,
}

fn read_index(
    token: &str,
    count: usize,
) -> Result<usize, String> {
    let raw = token.split('/').next().unwrap_or("");

    let index = raw
        .parse::<isize>()
        .map_err(|_| format!("Indice OBJ invalido: {token}"))?;

    let zero_based = if index > 0 {
        index - 1
    } else {
        count as isize + index
    };

    if index == 0
        || zero_based < 0
        || zero_based >= count as isize
    {
        return Err(format!("Indice fuera de rango: {token}"));
    }

    Ok(zero_based as usize)
}

pub fn load_obj(
    path: impl AsRef<Path>,
) -> Result<Model, String> {
    let file = File::open(path)
        .map_err(|e| e.to_string())?;

    let mut model = Model {
        vertices: Vec::new(),
        faces: Vec::new(),
    };

    let mut material = "Hull White".to_string();

    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let mut parts = line.split_whitespace();

        match parts.next() {
            Some("v") => {
                let nums: Vec<f32> = parts
                    .take(3)
                    .map(|s| s.parse::<f32>())
                    .collect::<Result<_, _>>()
                    .map_err(|e| {
                        format!("Linea {}: {e}", line_no + 1)
                    })?;

                if nums.len() == 3 {
                    model.vertices.push(Vec3::new(
                        nums[0],
                        nums[1],
                        nums[2],
                    ));
                }
            }

            Some("usemtl") => {
                material = parts.collect::<Vec<_>>().join(" ");
            }

            Some("f") => {
                let ids = parts
                    .map(|t| {
                        read_index(
                            t,
                            model.vertices.len(),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                for i in 1..ids.len().saturating_sub(1) {
                    model.faces.push(Face {
                        indices: [
                            ids[0],
                            ids[i],
                            ids[i + 1],
                        ],
                        material: material.clone(),
                    });
                }
            }

            _ => {}
        }
    }

    if model.vertices.is_empty() || model.faces.is_empty() {
        return Err("OBJ sin vertices o caras".into());
    }

    Ok(model)
}
