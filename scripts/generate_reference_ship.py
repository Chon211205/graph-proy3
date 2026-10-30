"""Generate a detailed hard-surface interceptor from the supplied concept image.

The mesh uses only vertices, material groups and polygon faces understood by the
game's OBJ loader. Run from the repository root with Python 3.11+.
"""

from __future__ import annotations

import math
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OBJ_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2.obj"
MTL_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2.mtl"


class Mesh:
    def __init__(self) -> None:
        self.vertices: list[tuple[float, float, float]] = []
        self.parts: list[tuple[str, str, list[tuple[int, ...]]]] = []

    def part(self, name: str, material: str, vertices, faces) -> None:
        start = len(self.vertices) + 1
        self.vertices.extend(tuple(map(float, vertex)) for vertex in vertices)
        self.parts.append(
            (name, material, [tuple(start + index for index in face) for face in faces])
        )

    def extrude_xy(self, name: str, material: str, outline, z0: float, z1: float) -> None:
        count = len(outline)
        vertices = [(x, y, z0) for x, y in outline] + [(x, y, z1) for x, y in outline]
        faces = [tuple(range(count - 1, -1, -1)), tuple(range(count, count * 2))]
        for index in range(count):
            nxt = (index + 1) % count
            faces.append((index, nxt, count + nxt, count + index))
        self.part(name, material, vertices, faces)

    def extrude_xz(self, name: str, material: str, outline, y0: float, y1: float) -> None:
        count = len(outline)
        vertices = [(x, y0, z) for x, z in outline] + [(x, y1, z) for x, z in outline]
        faces = [tuple(range(count - 1, -1, -1)), tuple(range(count, count * 2))]
        for index in range(count):
            nxt = (index + 1) % count
            faces.append((index, nxt, count + nxt, count + index))
        self.part(name, material, vertices, faces)

    def box(self, name: str, material: str, center, size, taper: float = 1.0) -> None:
        cx, cy, cz = center
        sx, sy, sz = (value * 0.5 for value in size)
        vertices = [
            (cx - sx, cy - sy * taper, cz - sz * taper),
            (cx - sx, cy + sy * taper, cz - sz * taper),
            (cx - sx, cy + sy * taper, cz + sz * taper),
            (cx - sx, cy - sy * taper, cz + sz * taper),
            (cx + sx, cy - sy, cz - sz),
            (cx + sx, cy + sy, cz - sz),
            (cx + sx, cy + sy, cz + sz),
            (cx + sx, cy - sy, cz + sz),
        ]
        faces = [
            (0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4),
            (3, 7, 6, 2), (0, 4, 7, 3), (1, 2, 6, 5),
        ]
        self.part(name, material, vertices, faces)

    def tube(self, name: str, material: str, x0: float, x1: float, y: float, z: float,
             radius: float, sides: int = 24) -> None:
        vertices = []
        for x in (x0, x1):
            vertices.extend(
                (x, y + math.cos(i * math.tau / sides) * radius,
                 z + math.sin(i * math.tau / sides) * radius)
                for i in range(sides)
            )
        faces = [tuple(range(sides - 1, -1, -1)), tuple(range(sides, sides * 2))]
        for index in range(sides):
            nxt = (index + 1) % sides
            faces.append((index, nxt, sides + nxt, sides + index))
        self.part(name, material, vertices, faces)

    def write(self, obj_path: Path = OBJ_PATH, mtl_path: Path = MTL_PATH) -> None:
        lines = ["# Detailed blue interceptor", f"mtllib {mtl_path.name}", ""]
        lines.extend(f"v {x:.5f} {y:.5f} {z:.5f}" for x, y, z in self.vertices)
        lines.append("")
        for name, material, faces in self.parts:
            smooth = "1" if name.startswith(("fuselaje", "cabina", "motor", "escape")) else "off"
            lines.extend((f"o {name}", f"g {name}", f"usemtl {material}", f"s {smooth}"))
            lines.extend("f " + " ".join(map(str, face)) for face in faces)
            lines.append("")
        obj_path.write_text("\n".join(lines), encoding="utf-8")


def add_fuselage(mesh: Mesh) -> None:
    # Dense elliptical rings produce a smooth aerospace hull while preserving
    # a slightly flattened underside and crisp hard-surface proportions.
    sections = [
        (-7.2, 0.03, 0.02), (-5.8, 0.34, 0.22), (-3.3, 0.82, 0.48),
        (-0.7, 1.18, 0.65), (2.0, 1.05, 0.58), (4.2, 0.62, 0.42),
        (5.2, 0.18, 0.20),
    ]
    sides = 32
    vertices = []
    for x, ry, rz in sections:
        for index in range(sides):
            angle = math.tau * index / sides
            z = math.cos(angle) * rz
            if z < 0:
                z *= 0.82
            vertices.append((x, math.sin(angle) * ry, z))
    faces = [tuple(range(sides - 1, -1, -1))]
    for section in range(len(sections) - 1):
        a, b = section * sides, (section + 1) * sides
        for index in range(sides):
            nxt = (index + 1) % sides
            faces.append((a + index, a + nxt, b + nxt, b + index))
    faces.append(tuple(range((len(sections) - 1) * sides, len(sections) * sides)))
    mesh.part("fuselaje_principal", "hull_white", vertices, faces)


def add_canopy(mesh: Mesh) -> None:
    sections = [
        (-3.65, 0.04, 0.04, 0.48),
        (-3.15, 0.38, 0.18, 0.56),
        (-2.15, 0.58, 0.34, 0.62),
        (-0.95, 0.54, 0.29, 0.62),
        (0.15, 0.30, 0.16, 0.58),
        (0.58, 0.03, 0.03, 0.54),
    ]
    sides = 20
    vertices = []
    for x, ry, rz, center_z in sections:
        for index in range(sides):
            angle = math.tau * index / sides
            vertices.append((x, math.sin(angle) * ry, center_z + math.cos(angle) * rz))
    faces = [tuple(range(sides - 1, -1, -1))]
    for section in range(len(sections) - 1):
        a, b = section * sides, (section + 1) * sides
        for index in range(sides):
            nxt = (index + 1) % sides
            faces.append((a + index, a + nxt, b + nxt, b + index))
    faces.append(tuple(range((len(sections) - 1) * sides, len(sections) * sides)))
    mesh.part("cabina_curva", "cockpit_blue", vertices, faces)


def build() -> Mesh:
    mesh = Mesh()
    add_fuselage(mesh)

    # Broad swept wings and the small dark nose/wing-tip panels.
    left_wing = [(-2.8, 0.55), (0.3, 1.0), (4.7, 4.7), (5.35, 4.9), (3.9, 1.0)]
    right_wing = [(x, -y) for x, y in left_wing]
    mesh.extrude_xy("ala_izquierda", "wing_metal", left_wing, -0.10, 0.15)
    mesh.extrude_xy("ala_derecha", "wing_metal", right_wing, -0.10, 0.15)
    mesh.extrude_xy("punta_izquierda", "warning_yellow", [(4.65, 4.62), (5.35, 4.9), (5.08, 4.25)], -0.08, 0.17)
    mesh.extrude_xy("punta_derecha", "warning_yellow", [(x, -y) for x, y in [(4.65, 4.62), (5.35, 4.9), (5.08, 4.25)]], -0.08, 0.17)
    mesh.extrude_xy("nariz_inferior", "dark_metal", [(-7.2, 0), (-5.1, 0.38), (-5.5, 0), (-5.1, -0.38)], -0.23, -0.05)

    # Cockpit and frame: a dense curved canopy rather than a faceted prism.
    add_canopy(mesh)
    mesh.extrude_xy("marco_cabina", "dark_metal", [(-3.15, -0.58), (-3.85, 0), (-3.0, 0.58), (-2.72, 0.48), (-3.48, 0), (-2.86, -0.48)], 0.44, 0.52)

    # Four characteristic blue fins, mirrored around the centerline.
    outer_fin = [(-0.4, 0.1), (1.0, 0.2), (3.9, 3.35), (2.9, 0.15)]
    inner_fin = [(-1.4, 0.18), (-0.4, 0.28), (1.7, 2.75), (1.25, 0.18)]
    for side, sign in (("izquierda", 1), ("derecha", -1)):
        mesh.extrude_xz(f"aleta_exterior_{side}", "fin_blue", outer_fin, sign * 1.52 - 0.10, sign * 1.52 + 0.10)
        mesh.extrude_xz(f"aleta_interior_{side}", "fin_blue", inner_fin, sign * 0.72 - 0.09, sign * 0.72 + 0.09)

    # Raised engine pods and cyan exhaust cores.
    for side, y in (("izquierdo", 2.22), ("derecho", -2.22)):
        mesh.tube(f"motor_{side}", "dark_metal", 1.8, 4.05, y, 0.45, 0.42)
        mesh.tube(f"escape_{side}", "engine_cyan", 4.06, 4.22, y, 0.45, 0.24)
        mesh.box(f"cubierta_motor_{side}", "hull_white", (2.75, y, 0.66), (1.55, 0.95, 0.30), taper=0.72)

    # Underside blue stabilizers and small hull armor panels.
    stabilizer = [(0.2, 0.0), (2.8, -0.08), (1.85, -1.05), (0.65, -0.55)]
    for side, y in (("izquierdo", 0.74), ("derecho", -0.74)):
        mesh.extrude_xz(f"estabilizador_{side}", "fin_blue", stabilizer, y - 0.09, y + 0.09)
    mesh.box("placa_central", "wing_metal", (1.65, 0, -0.58), (2.9, 1.45, 0.18), taper=0.72)
    mesh.box("espina_superior", "hull_white", (1.8, 0, 0.66), (3.1, 0.65, 0.22), taper=0.65)
    return mesh


MTL = """# Materials for arwing2.obj
newmtl hull_white
Kd 0.86 0.89 0.91
Ks 0.40 0.44 0.48
Ns 90
illum 2

newmtl wing_metal
Kd 0.38 0.42 0.45
Ks 0.35 0.38 0.42
Ns 70
illum 2

newmtl fin_blue
Kd 0.08 0.18 0.48
Ks 0.30 0.40 0.70
Ns 110
illum 2

newmtl cockpit_blue
Kd 0.04 0.13 0.24
Ks 0.55 0.72 0.85
Ns 180
d 0.82
Ni 1.45
illum 9

newmtl dark_metal
Kd 0.06 0.08 0.11
Ks 0.25 0.28 0.33
Ns 80
illum 2

newmtl engine_cyan
Kd 0.05 0.75 0.95
Ke 0.02 0.40 0.65
Ks 0.25 0.85 1.00
Ns 160
illum 2

newmtl warning_yellow
Kd 0.92 0.70 0.04
Ks 0.30 0.25 0.05
Ns 60
illum 2
"""


if __name__ == "__main__":
    OBJ_PATH.parent.mkdir(parents=True, exist_ok=True)
    build().write()
    MTL_PATH.write_text(MTL, encoding="utf-8")
    print(f"Generated {OBJ_PATH.relative_to(ROOT)}")
