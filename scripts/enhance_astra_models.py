"""Append detailed hard-surface components to the three ASTRA OBJ models."""

from __future__ import annotations

import math
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ASTRA_DIR = ROOT / "assets" / "vehicles" / "astra"


class Appender:
    def __init__(self, source: Path):
        self.source = source
        self.text = source.read_text(encoding="utf-8", errors="ignore").rstrip()
        self.offset = sum(line.startswith("v ") for line in self.text.splitlines())
        self.parts: list[str] = []

    def part(self, name: str, material: str, vertices, faces, smooth=True):
        lines = ["", f"o {name}", f"g {name}", f"usemtl {material}", f"s {1 if smooth else 0}"]
        lines.extend(f"v {x:.6f} {y:.6f} {z:.6f}" for x, y, z in vertices)
        lines.extend(
            "f " + " ".join(str(self.offset + index + 1) for index in face)
            for face in faces
        )
        self.offset += len(vertices)
        self.parts.extend(lines)

    def sphere(self, name, material, center, scale, slices=24, stacks=12):
        cx, cy, cz = center
        sx, sy, sz = scale
        vertices = []
        for stack in range(stacks + 1):
            latitude = math.pi * stack / stacks - math.pi / 2
            for segment in range(slices):
                angle = math.tau * segment / slices
                vertices.append((
                    cx + math.cos(latitude) * math.cos(angle) * sx,
                    cy + math.sin(latitude) * sy,
                    cz + math.cos(latitude) * math.sin(angle) * sz,
                ))
        faces = []
        for stack in range(stacks):
            for segment in range(slices):
                nxt = (segment + 1) % slices
                a = stack * slices + segment
                b = stack * slices + nxt
                c = (stack + 1) * slices + nxt
                d = (stack + 1) * slices + segment
                faces.extend(((a, b, c), (a, c, d)))
        self.part(name, material, vertices, faces)

    def tube_z(self, name, material, center, radius, depth, segments=24):
        cx, cy, cz = center
        vertices = []
        for z in (cz - depth / 2, cz + depth / 2):
            vertices.extend((cx + math.cos(math.tau * i / segments) * radius,
                             cy + math.sin(math.tau * i / segments) * radius, z)
                            for i in range(segments))
        faces = [tuple(range(segments - 1, -1, -1)), tuple(range(segments, segments * 2))]
        for i in range(segments):
            n = (i + 1) % segments
            faces.append((i, n, segments + n, segments + i))
        self.part(name, material, vertices, faces)

    def tube_x(self, name, material, center, radius, depth, segments=24):
        cx, cy, cz = center
        vertices = []
        for x in (cx - depth / 2, cx + depth / 2):
            vertices.extend((x, cy + math.cos(math.tau * i / segments) * radius,
                             cz + math.sin(math.tau * i / segments) * radius)
                            for i in range(segments))
        faces = [tuple(range(segments - 1, -1, -1)), tuple(range(segments, segments * 2))]
        for i in range(segments):
            n = (i + 1) % segments
            faces.append((i, n, segments + n, segments + i))
        self.part(name, material, vertices, faces)

    def torus_z(self, name, material, center, major, minor, major_segments=32, minor_segments=8):
        cx, cy, cz = center
        vertices = []
        for ring in range(major_segments):
            a = math.tau * ring / major_segments
            for segment in range(minor_segments):
                b = math.tau * segment / minor_segments
                r = major + minor * math.cos(b)
                vertices.append((cx + r * math.cos(a), cy + r * math.sin(a), cz + minor * math.sin(b)))
        faces = []
        for ring in range(major_segments):
            nr = (ring + 1) % major_segments
            for segment in range(minor_segments):
                ns = (segment + 1) % minor_segments
                faces.append((ring * minor_segments + segment, nr * minor_segments + segment,
                              nr * minor_segments + ns, ring * minor_segments + ns))
        self.part(name, material, vertices, faces)

    def box(self, name, material, center, size):
        cx, cy, cz = center
        x, y, z = (value / 2 for value in size)
        vertices = [(cx + dx, cy + dy, cz + dz) for dx, dy, dz in (
            (-x, -y, -z), (-x, y, -z), (-x, y, z), (-x, -y, z),
            (x, -y, -z), (x, y, -z), (x, y, z), (x, -y, z),
        )]
        faces = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4),
                 (3, 7, 6, 2), (0, 4, 7, 3), (1, 2, 6, 5)]
        self.part(name, material, vertices, faces, smooth=False)

    def write(self, target: Path):
        header = self.text.replace(self.source.with_suffix(".mtl").name, target.with_suffix(".mtl").name, 1)
        target.write_text(header + "\n" + "\n".join(self.parts) + "\n", encoding="utf-8")


MTL = """# ASTRA detailed materials
newmtl Casco_blanco
Kd 0.88 0.92 0.96
Ks 0.65 0.70 0.75
Ns 140
illum 2

newmtl Azul_cobalto
Kd 0.04 0.26 0.78
Ks 0.30 0.48 0.90
Ns 130
illum 2

newmtl Azul_profundo
Kd 0.02 0.08 0.26
Ks 0.20 0.30 0.55
Ns 95
illum 2

newmtl Vidrio_oscuro
Kd 0.03 0.16 0.24
Ks 0.60 0.78 0.92
Ns 190
d 0.80
Ni 1.45
illum 9

newmtl Luz_cian
Kd 0.04 0.78 1.00
Ke 0.02 0.55 0.90
Ns 180
illum 2

newmtl Luz_roja
Kd 1.00 0.04 0.06
Ke 0.65 0.01 0.01
Ns 150
illum 2

newmtl Titanio
Kd 0.34 0.40 0.48
Ks 0.62 0.68 0.75
Ns 115
illum 2

newmtl Grafito
Kd 0.035 0.045 0.065
Ks 0.24 0.28 0.34
Ns 75
illum 2
"""


def spaceship() -> Appender:
    model = Appender(ASTRA_DIR / "nave_astra.obj")
    for side, x in (("L", -3.13), ("R", 3.13)):
        model.tube_z(f"Astra_Engine_{side}", "Grafito", (x, 0.05, -5.15), 0.48, 0.80)
        model.torus_z(f"Astra_Engine_Guard_{side}", "Titanio", (x, 0.05, -5.48), 0.52, 0.07)
        model.tube_z(f"Astra_Engine_Core_{side}", "Luz_cian", (x, 0.05, -5.58), 0.30, 0.08)
        model.box(f"Astra_Wing_Rail_{side}", "Azul_profundo", (x * 0.64, 1.25, -0.4), (0.16, 0.18, 5.4))
    for side, x in (("L", -5.38), ("R", 5.38)):
        model.sphere(f"Astra_Wingtip_{side}", "Luz_cian", (x, 0.02, -0.15), (0.18, 0.14, 0.28), 18, 9)
    model.box("Astra_Dorsal_Armor", "Titanio", (0, 1.63, -0.45), (1.35, 0.18, 3.4))
    return model


def car() -> Appender:
    model = Appender(ASTRA_DIR / "astra_carro.obj")
    for axle, z in (("Front", 2.15), ("Rear", -1.85)):
        for side, x in (("L", -1.50), ("R", 1.50)):
            model.tube_x(f"Astra_Rim_{axle}_{side}", "Titanio", (x, 0.48, z), 0.40, 0.12, 28)
            model.tube_x(f"Astra_Hub_{axle}_{side}", "Luz_cian", (x * 1.01, 0.48, z), 0.16, 0.14, 20)
    for side, x in (("L", -0.72), ("R", 0.72)):
        model.sphere(f"Astra_Headlight_{side}", "Luz_cian", (x, 0.78, 3.12), (0.28, 0.16, 0.10), 20, 10)
        model.sphere(f"Astra_Taillight_{side}", "Luz_roja", (x, 0.76, -2.68), (0.24, 0.14, 0.10), 20, 10)
        model.tube_z(f"Astra_Car_Exhaust_{side}", "Grafito", (x, 0.40, -2.82), 0.19, 0.34, 20)
        model.tube_z(f"Astra_Car_Core_{side}", "Luz_cian", (x, 0.40, -3.01), 0.12, 0.06, 18)
    model.box("Astra_Front_Grille", "Grafito", (0, 0.46, 3.14), (1.05, 0.28, 0.10))
    model.box("Astra_Center_Stripe", "Azul_profundo", (0, 1.72, 0.25), (0.24, 0.08, 3.8))
    return model


def submarine() -> Appender:
    model = Appender(ASTRA_DIR / "astra_submarino.obj")
    for side, x in (("L", -1.53), ("R", 1.53)):
        model.tube_z(f"Astra_Sub_Propeller_{side}", "Grafito", (x, 0.15, -3.72), 0.46, 0.62)
        model.torus_z(f"Astra_Sub_Guard_{side}", "Titanio", (x, 0.15, -3.94), 0.64, 0.08, 36)
        model.tube_z(f"Astra_Sub_Core_{side}", "Luz_cian", (x, 0.15, -4.03), 0.26, 0.08)
        model.sphere(f"Astra_Porthole_{side}", "Vidrio_oscuro", (x * 1.27, 0.72, 0.65), (0.16, 0.38, 0.38), 22, 11)
    for side, x in (("L", -0.68), ("R", 0.68)):
        model.sphere(f"Astra_Sub_Headlight_{side}", "Luz_cian", (x, 0.34, 3.90), (0.24, 0.20, 0.12), 20, 10)
    model.box("Astra_Sub_Dorsal_Rail", "Azul_profundo", (0, 1.96, 0.10), (0.22, 0.16, 3.6))
    model.tube_z("Astra_Sub_Sonar", "Titanio", (0, 2.34, -0.20), 0.18, 0.52, 20)
    return model


def generate(name: str, builder) -> None:
    target = ASTRA_DIR / name
    builder().write(target)
    target.with_suffix(".mtl").write_text(MTL, encoding="utf-8")
    print(f"Generated {target.relative_to(ROOT)}")


if __name__ == "__main__":
    generate("nave_astra_detallada.obj", spaceship)
    generate("astra_carro_detallado.obj", car)
    generate("astra_submarino_detallado.obj", submarine)
