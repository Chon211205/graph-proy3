"""Build a long, three-sector underwater level from the detailed source environment."""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
LEVEL_DIR = ROOT / "assets" / "levels"
SOURCE = LEVEL_DIR / "waterlevel.obj"
TARGET = LEVEL_DIR / "waterlevel_extendido.obj"
SECTOR_LENGTH = 180.0
SECTOR_COUNT = 3


def read_source():
    vertices = []
    commands = []
    for line in SOURCE.read_text(encoding="utf-8", errors="ignore").splitlines():
        if line.startswith("v "):
            vertices.append(tuple(map(float, line.split()[1:4])))
        elif line.startswith("mtllib ") or line.startswith("#") or not line.strip():
            continue
        else:
            commands.append(line)
    return vertices, commands


def transformed_vertex(vertex, sector):
    x, y, z = vertex
    x += sector * SECTOR_LENGTH
    y *= 1.25
    # Alternate the reef layout without changing the safe corridor dimensions.
    if sector == 1:
        y = -y
    elif sector == 2:
        y *= 0.96
    return x, y, z


def append_box(lines, state, name, material, center, size):
    cx, cy, cz = center
    sx, sy, sz = (value * 0.5 for value in size)
    vertices = [
        (cx - sx, cy - sy, cz - sz), (cx - sx, cy + sy, cz - sz),
        (cx - sx, cy + sy, cz + sz), (cx - sx, cy - sy, cz + sz),
        (cx + sx, cy - sy, cz - sz), (cx + sx, cy + sy, cz - sz),
        (cx + sx, cy + sy, cz + sz), (cx + sx, cy - sy, cz + sz),
    ]
    faces = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4),
             (3, 7, 6, 2), (0, 4, 7, 3), (1, 2, 6, 5)]
    lines.extend((f"o {name}", f"g {name}", f"usemtl {material}", "s 0"))
    lines.extend(f"v {x:.6f} {y:.6f} {z:.6f}" for x, y, z in vertices)
    lines.extend(
        "f " + " ".join(str(state[0] + index + 1) for index in face)
        for face in faces
    )
    state[0] += len(vertices)


def append_crystal(lines, state, name, center, scale):
    cx, cy, cz = center
    sx, sy, sz = scale
    vertices = [
        (cx, cy, cz + sz), (cx, cy, cz - sz),
        (cx + sx, cy, cz), (cx - sx, cy, cz),
        (cx, cy + sy, cz), (cx, cy - sy, cz),
    ]
    faces = [(0, 2, 4), (0, 4, 3), (0, 3, 5), (0, 5, 2),
             (1, 4, 2), (1, 3, 4), (1, 5, 3), (1, 2, 5)]
    lines.extend((f"o {name}", f"g {name}", "usemtl Crystal_Cyan", "s 1"))
    lines.extend(f"v {x:.6f} {y:.6f} {z:.6f}" for x, y, z in vertices)
    lines.extend(
        "f " + " ".join(str(state[0] + index + 1) for index in face)
        for face in faces
    )
    state[0] += len(vertices)


def build():
    source_vertices, commands = read_source()
    lines = [
        "# Extended underwater level generated from waterlevel.obj",
        "# Three visually varied sectors, collision obstacles and crystal gates",
        "mtllib waterlevel_extendido.mtl",
    ]
    vertex_offset = 0
    for sector in range(SECTOR_COUNT):
        lines.append(f"# --- SECTOR {sector + 1} ---")
        lines.extend(
            f"v {x:.6f} {y:.6f} {z:.6f}"
            for x, y, z in (transformed_vertex(vertex, sector) for vertex in source_vertices)
        )
        skip_upper_group = False
        for command in commands:
            if command.startswith(("o ", "g ")):
                prefix, name = command.split(maxsplit=1)
                skip_upper_group = name.startswith("CAVES_AND_ARCHES")
                if not skip_upper_group:
                    lines.append(f"{prefix} {name}_SECTOR_{sector + 1}")
            elif skip_upper_group:
                continue
            elif command.startswith("f "):
                tokens = []
                for token in command.split()[1:]:
                    pieces = token.split("/")
                    pieces[0] = str(int(pieces[0]) + vertex_offset)
                    tokens.append("/".join(pieces))
                lines.append("f " + " ".join(tokens))
            else:
                lines.append(command)
        vertex_offset += len(source_vertices)

    state = [vertex_offset]
    # Side pillars create readable slalom moments without blocking the space above.
    for index, x in enumerate([-18.0, 58.0, 128.0, 202.0, 276.0, 350.0, 422.0]):
        side = -1.0 if index % 2 == 0 else 1.0
        append_box(
            lines, state, f"obstacle_reef_pillar_{index + 1:02d}", "Deep_Cave_Rock",
            (x, side * 7.3, -0.7), (3.2, 4.2, 8.6),
        )
        append_crystal(
            lines, state, f"LONG_LEVEL_CRYSTAL_{index + 1:02d}",
            (x - 5.0, -side * 7.0, -2.8), (0.8, 0.8, 2.1),
        )

    TARGET.write_text("\n".join(lines) + "\n", encoding="utf-8")
    TARGET.with_suffix(".mtl").write_text(MTL, encoding="utf-8")
    print(f"Generated {TARGET.relative_to(ROOT)}")


MTL = """# Extended underwater level materials
newmtl Underwater_Sand
Kd 0.54 0.47 0.25
newmtl Underwater_Rock
Kd 0.20 0.34 0.39
newmtl Deep_Cave_Rock
Kd 0.08 0.14 0.19
newmtl Blue_Stone
Kd 0.15 0.36 0.52
newmtl Seaweed_Green
Kd 0.04 0.34 0.20
newmtl Bright_Seaweed
Kd 0.10 0.62 0.32
newmtl Pink_Coral
Kd 0.90 0.24 0.56
newmtl Orange_Coral
Kd 0.94 0.34 0.10
newmtl Purple_Coral
Kd 0.48 0.18 0.72
newmtl Cyan_Coral
Kd 0.08 0.78 0.82
newmtl Crystal_Cyan
Kd 0.08 0.86 0.94
Ke 0.02 0.48 0.62
illum 2
"""


if __name__ == "__main__":
    build()
