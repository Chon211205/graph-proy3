"""Generate the aquatic transformation of arwing2.obj."""

from __future__ import annotations

import math
from pathlib import Path

from generate_reference_ship import MTL, Mesh


ROOT = Path(__file__).resolve().parents[1]
OBJ_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2_submarino.obj"
MTL_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2_submarino.mtl"


def loft(mesh: Mesh, name: str, material: str, sections, sides: int = 28) -> None:
    vertices = []
    for x, y, z, ry, rz in sections:
        for index in range(sides):
            angle = math.tau * index / sides
            vertices.append((x, y + math.sin(angle) * ry, z + math.cos(angle) * rz))
    faces = [tuple(range(sides - 1, -1, -1))]
    for section in range(len(sections) - 1):
        a, b = section * sides, (section + 1) * sides
        for index in range(sides):
            nxt = (index + 1) % sides
            faces.append((a + index, a + nxt, b + nxt, b + index))
    faces.append(tuple(range((len(sections) - 1) * sides, len(sections) * sides)))
    mesh.part(name, material, vertices, faces)


def torus_x(mesh: Mesh, name: str, material: str, center, major: float, minor: float,
            major_segments: int = 24, minor_segments: int = 8) -> None:
    cx, cy, cz = center
    vertices = []
    for ring in range(major_segments):
        a = math.tau * ring / major_segments
        for segment in range(minor_segments):
            b = math.tau * segment / minor_segments
            radius = major + minor * math.cos(b)
            vertices.append((cx + minor * math.sin(b), cy + radius * math.cos(a), cz + radius * math.sin(a)))
    faces = []
    for ring in range(major_segments):
        next_ring = (ring + 1) % major_segments
        for segment in range(minor_segments):
            nxt = (segment + 1) % minor_segments
            faces.append((
                ring * minor_segments + segment,
                next_ring * minor_segments + segment,
                next_ring * minor_segments + nxt,
                ring * minor_segments + nxt,
            ))
    mesh.part(name, material, vertices, faces)


def propeller(mesh: Mesh, side: str, y: float) -> None:
    x, z = 4.48, -0.02
    mesh.tube(f"eje_helice_{side}", "dark_metal", 4.12, 4.58, y, z, 0.11, 16)
    torus_x(mesh, f"aro_helice_{side}", "fin_blue", (x, y, z), 0.55, 0.075)
    for index, angle in enumerate((0.0, math.pi / 2, math.pi, math.pi * 1.5)):
        dy, dz = math.cos(angle), math.sin(angle)
        outline = [
            (x - 0.06, y + dy * 0.08, z + dz * 0.08),
            (x - 0.04, y + dy * 0.48 - dz * 0.15, z + dz * 0.48 + dy * 0.15),
            (x + 0.04, y + dy * 0.40 + dz * 0.14, z + dz * 0.40 - dy * 0.14),
            (x + 0.06, y + dy * 0.08, z + dz * 0.08),
        ]
        mesh.part(f"aspas_{side}_{index}", "engine_cyan", outline, [(0, 1, 2, 3)])


def build() -> Mesh:
    mesh = Mesh()
    loft(mesh, "casco_principal", "hull_white", [
        (-6.2, 0, 0, 0.03, 0.03), (-5.25, 0, 0, 0.46, 0.40),
        (-3.1, 0, 0, 0.95, 0.72), (-0.4, 0, 0, 1.18, 0.86),
        (2.1, 0, 0, 1.02, 0.72), (3.65, 0, 0, 0.66, 0.50),
        (4.25, 0, 0, 0.24, 0.24),
    ])

    # Glazed command canopy and blue armored belly.
    loft(mesh, "cabina_submarina", "cockpit_blue", [
        (-3.15, 0, 0.54, 0.05, 0.04), (-2.35, 0, 0.64, 0.52, 0.30),
        (-0.75, 0, 0.70, 0.66, 0.39), (0.35, 0, 0.58, 0.34, 0.20),
    ], 20)
    mesh.extrude_xy("blindaje_inferior", "fin_blue", [
        (-3.9, -0.62), (-2.2, -1.00), (1.9, -0.82), (3.2, -0.42),
        (1.4, 0.42), (-2.5, 0.48),
    ], -0.76, -0.62)

    # Compact diving planes preserve the swept Arwing silhouette.
    left_plane = [(-2.8, 0.58), (-0.4, 0.88), (2.65, 3.05), (3.25, 3.02), (2.05, 0.72)]
    mesh.extrude_xy("aleta_buceo_izquierda", "fin_blue", left_plane, -0.10, 0.12)
    mesh.extrude_xy("aleta_buceo_derecha", "fin_blue", [(x, -y) for x, y in left_plane], -0.10, 0.12)
    mesh.extrude_xz("timón_superior", "fin_blue", [(1.0, 0.45), (3.35, 0.30), (2.95, 1.65), (1.65, 1.28)], -0.11, 0.11)
    mesh.extrude_xz("timón_inferior", "fin_blue", [(1.55, -0.52), (3.40, -0.30), (2.95, -1.12), (2.05, -0.95)], -0.09, 0.09)

    # Conning tower, periscope, sensor cap and forward lamps.
    mesh.box("torre_mando", "wing_metal", (0.35, 0, 0.93), (1.55, 0.72, 0.50), taper=0.72)
    mesh.tube("mastil_periscopio", "dark_metal", -0.02, 0.28, 0, 1.48, 0.11, 16)
    mesh.box("cabeza_periscopio", "dark_metal", (-0.18, 0, 1.61), (0.48, 0.25, 0.22), taper=0.65)
    for side, y in (("izquierdo", 0.60), ("derecho", -0.60)):
        mesh.tube(f"foco_{side}", "warning_yellow", -4.58, -4.36, y, 0.02, 0.19, 20)

    # Side ballast pods and protected twin propellers.
    for side, y in (("izquierdo", 1.55), ("derecho", -1.55)):
        loft(mesh, f"flotador_{side}", "wing_metal", [
            (-2.55, y, -0.22, 0.18, 0.18), (-1.80, y, -0.22, 0.36, 0.34),
            (2.90, y, -0.18, 0.38, 0.34), (3.85, y, -0.12, 0.18, 0.18),
        ], 18)
        propeller(mesh, side, y)

    # Cyan navigation strips along the ballast pods.
    mesh.extrude_xy("luz_lateral_izquierda", "engine_cyan", [(-1.8, 1.88), (2.8, 1.88), (2.8, 1.82), (-1.8, 1.82)], -0.20, -0.08)
    mesh.extrude_xy("luz_lateral_derecha", "engine_cyan", [(-1.8, -1.88), (2.8, -1.88), (2.8, -1.82), (-1.8, -1.82)], -0.20, -0.08)
    return mesh


if __name__ == "__main__":
    OBJ_PATH.parent.mkdir(parents=True, exist_ok=True)
    build().write(OBJ_PATH, MTL_PATH)
    MTL_PATH.write_text(MTL, encoding="utf-8")
    print(f"Generated {OBJ_PATH.relative_to(ROOT)}")
