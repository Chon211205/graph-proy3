"""Create detailed Swordfish II/Cowboy vehicle variants without replacing the originals."""

from __future__ import annotations

import math
from pathlib import Path

from enhance_astra_models import Appender


ROOT = Path(__file__).resolve().parents[1]
COWBOY_DIR = ROOT / "assets" / "vehicles" / "cowboy"


class CowboyAppender(Appender):
    def tube_y(self, name, material, center, radius, depth, segments=28):
        cx, cy, cz = center
        vertices = []
        for y in (cy - depth / 2, cy + depth / 2):
            vertices.extend(
                (cx + math.cos(math.tau * i / segments) * radius, y,
                 cz + math.sin(math.tau * i / segments) * radius)
                for i in range(segments)
            )
        faces = [tuple(range(segments - 1, -1, -1)), tuple(range(segments, segments * 2))]
        for i in range(segments):
            nxt = (i + 1) % segments
            faces.append((i, nxt, segments + nxt, segments + i))
        self.part(name, material, vertices, faces)

    def torus_y(self, name, material, center, major, minor, major_segments=36, minor_segments=8):
        cx, cy, cz = center
        vertices = []
        for ring in range(major_segments):
            a = math.tau * ring / major_segments
            for segment in range(minor_segments):
                b = math.tau * segment / minor_segments
                radius = major + minor * math.cos(b)
                vertices.append((cx + radius * math.cos(a), cy + minor * math.sin(b),
                                 cz + radius * math.sin(a)))
        faces = []
        for ring in range(major_segments):
            nr = (ring + 1) % major_segments
            for segment in range(minor_segments):
                ns = (segment + 1) % minor_segments
                faces.append((ring * minor_segments + segment, nr * minor_segments + segment,
                              nr * minor_segments + ns, ring * minor_segments + ns))
        self.part(name, material, vertices, faces)


MTL = """# Detailed Cowboy / Swordfish II materials
newmtl Amarillo
Kd 0.92 0.54 0.025
Ks 0.88 0.72 0.34
Ns 210
illum 2

newmtl Rojo
Kd 0.72 0.022 0.018
Ks 0.82 0.30 0.20
Ns 195
illum 2

newmtl Gris_oscuro
Kd 0.09 0.105 0.125
Ks 0.82 0.86 0.90
Ns 230
illum 2

newmtl Cowboy_Yellow
Kd 0.95 0.61 0.04
Ks 0.90 0.76 0.38
Ns 220
illum 2

newmtl Cowboy_Red
Kd 0.78 0.035 0.025
Ks 0.86 0.34 0.24
Ns 205
illum 2

newmtl Cowboy_DarkRed
Kd 0.26 0.018 0.014
Ks 0.68 0.24 0.18
Ns 175
illum 2

newmtl Cowboy_Metal
Kd 0.36 0.40 0.43
Ks 0.78 0.82 0.84
Ns 165
illum 2

newmtl Cowboy_Graphite
Kd 0.035 0.040 0.050
Ks 0.25 0.28 0.32
Ns 85
illum 2

newmtl Cowboy_Glass
Kd 0.035 0.12 0.17
Ks 0.72 0.88 0.95
Ns 210
d 0.82
Ni 1.46
illum 9

newmtl Cowboy_OrangeGlow
Kd 1.00 0.24 0.025
Ke 0.92 0.12 0.01
Ns 180
illum 2

newmtl Cowboy_Cream
Kd 0.96 0.82 0.55
Ks 0.42 0.32 0.16
Ns 90
illum 2
"""


def spaceship() -> CowboyAppender:
    model = CowboyAppender(COWBOY_DIR / "nave_cowboyb.obj")
    # Characteristic twin rear engines, guards and hot cores.
    for side, x in (("L", -2.22), ("R", 2.22)):
        model.tube_y(f"Cowboy_Ship_Engine_{side}", "Cowboy_Graphite", (x, 2.72, 0.04), 0.43, 1.05)
        model.torus_y(f"Cowboy_Ship_Guard_{side}", "Cowboy_Metal", (x, 3.18, 0.04), 0.48, 0.075)
        model.tube_y(f"Cowboy_Ship_Core_{side}", "Cowboy_OrangeGlow", (x, 3.29, 0.04), 0.28, 0.08)
        model.box(f"Cowboy_Ship_WingRail_{side}", "Cowboy_DarkRed", (x * 1.25, -0.05, 0.20), (0.16, 4.8, 0.14))
        model.box(f"Cowboy_Ship_WingPanel_{side}", "Cowboy_Yellow", (x * 1.43, -0.15, 0.12), (0.30, 2.9, 0.08))
        model.sphere(f"Cowboy_Ship_WingLight_{side}", "Cowboy_OrangeGlow", (x * 1.78, 0.72, 0.10), (0.17, 0.28, 0.14), 20, 10)
    model.sphere("Cowboy_Ship_Canopy", "Cowboy_Glass", (0, -0.65, 0.72), (0.62, 1.22, 0.48), 32, 16)
    model.box("Cowboy_Ship_Spine", "Cowboy_Metal", (0, 0.75, 0.68), (0.22, 3.0, 0.13))
    model.box("Cowboy_Ship_NoseStripe", "Cowboy_Cream", (0, -3.05, 0.13), (0.18, 2.45, 0.10))
    model.tube_y("Cowboy_Ship_DorsalAntenna", "Cowboy_Metal", (0, 1.18, 1.42), 0.09, 0.65, 18)
    return model


def car() -> CowboyAppender:
    model = CowboyAppender(COWBOY_DIR / "cb_carro.obj")
    for axle, z in (("Front", 2.25), ("Rear", -1.75)):
        for side, x in (("L", -1.48), ("R", 1.48)):
            model.tube_x(f"Cowboy_Car_Rim_{axle}_{side}", "Cowboy_Metal", (x, 0.52, z), 0.39, 0.14, 30)
            model.tube_x(f"Cowboy_Car_Hub_{axle}_{side}", "Cowboy_Red", (x * 1.015, 0.52, z), 0.16, 0.16, 22)
    for side, x in (("L", -0.66), ("R", 0.66)):
        model.sphere(f"Cowboy_Car_Headlight_{side}", "Cowboy_Cream", (x, 0.70, 3.43), (0.25, 0.15, 0.09), 22, 11)
        model.sphere(f"Cowboy_Car_Taillight_{side}", "Cowboy_Red", (x, 0.66, -2.55), (0.24, 0.13, 0.09), 22, 11)
        model.tube_z(f"Cowboy_Car_Exhaust_{side}", "Cowboy_Graphite", (x, 0.36, -2.72), 0.19, 0.34, 22)
        model.tube_z(f"Cowboy_Car_Core_{side}", "Cowboy_OrangeGlow", (x, 0.36, -2.91), 0.12, 0.06, 20)
    model.sphere("Cowboy_Car_Canopy", "Cowboy_Glass", (0, 1.42, -0.05), (0.72, 0.36, 1.18), 30, 15)
    model.box("Cowboy_Car_Grille", "Cowboy_Graphite", (0, 0.43, 3.48), (1.12, 0.24, 0.08))
    model.box("Cowboy_Car_Stripe", "Cowboy_Yellow", (0, 1.66, 0.25), (0.18, 0.08, 3.25))
    model.box("Cowboy_Car_DarkRedTrim", "Cowboy_DarkRed", (0, 0.88, -2.56), (1.30, 0.14, 0.08))
    return model


def submarine() -> CowboyAppender:
    model = CowboyAppender(COWBOY_DIR / "cb_submarino.obj")
    for side, x in (("L", -1.70), ("R", 1.70)):
        model.tube_z(f"Cowboy_Sub_Propeller_{side}", "Cowboy_Graphite", (x, 0.06, -3.75), 0.43, 0.62, 28)
        model.torus_z(f"Cowboy_Sub_Guard_{side}", "Cowboy_Metal", (x, 0.06, -4.00), 0.61, 0.08, 38)
        model.tube_z(f"Cowboy_Sub_Core_{side}", "Cowboy_OrangeGlow", (x, 0.06, -4.10), 0.27, 0.08, 24)
        model.sphere(f"Cowboy_Sub_Porthole_{side}", "Cowboy_Glass", (x * 1.10, 0.70, 0.30), (0.18, 0.34, 0.34), 24, 12)
    for side, x in (("L", -0.65), ("R", 0.65)):
        model.sphere(f"Cowboy_Sub_Headlight_{side}", "Cowboy_Cream", (x, 0.28, 4.30), (0.24, 0.18, 0.10), 22, 11)
    model.sphere("Cowboy_Sub_Canopy", "Cowboy_Glass", (0, 1.52, 0.25), (0.72, 0.46, 1.18), 30, 15)
    model.box("Cowboy_Sub_DorsalRail", "Cowboy_DarkRed", (0, 1.94, 0.20), (0.20, 0.14, 3.3))
    model.box("Cowboy_Sub_YellowSpine", "Cowboy_Yellow", (0, 2.03, 0.36), (0.34, 0.10, 2.5))
    model.sphere("Cowboy_Sub_RedBeacon", "Cowboy_Red", (0, 2.36, -0.72), (0.16, 0.14, 0.16), 18, 9)
    model.tube_z("Cowboy_Sub_Sonar", "Cowboy_Metal", (0, 2.25, -0.25), 0.18, 0.48, 20)
    return model


def generate(filename: str, builder) -> None:
    target = COWBOY_DIR / filename
    builder().write(target)
    target.with_suffix(".mtl").write_text(MTL, encoding="utf-8")
    print(f"Generated {target.relative_to(ROOT)}")


if __name__ == "__main__":
    generate("nave_cowboyb_detallada.obj", spaceship)
    generate("cb_carro_detallado.obj", car)
    generate("cb_submarino_detallado.obj", submarine)
