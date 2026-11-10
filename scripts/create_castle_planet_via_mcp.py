"""Model the pasted castle/lava planet using the local Blender MCP add-on."""

from __future__ import annotations

import math
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from blender_mcp.bridge import BlenderBridge


ASSET_DIR = ROOT / "assets" / "planets"


def part(name, primitive, location, scale, color, material, rotation=(0, 0, 0)):
    return {
        "name": name,
        "primitive": primitive,
        "location": location,
        "rotation": rotation,
        "scale": scale,
        "color": color,
        "material_name": material,
    }


def castle_planet_components():
    rock = (0.22, 0.18, 0.18, 1.0)
    rock_light = (0.38, 0.31, 0.29, 1.0)
    lava = (1.0, 0.18, 0.015, 1.0)
    lava_hot = (1.0, 0.66, 0.04, 1.0)
    brick = (0.46, 0.43, 0.46, 1.0)
    brick_dark = (0.24, 0.20, 0.26, 1.0)
    purple = (0.50, 0.02, 0.82, 1.0)
    purple_hot = (1.0, 0.18, 1.0, 1.0)
    pieces = [
        part("Planeta_Roca", "sphere", (0, 0, -1.42), (2.80, 2.80, 2.25), rock, "Castle_Rock"),
        part("Plataforma_Lava_Curva", "cylinder", (0, 0, -1.10), (3.12, 3.12, 0.34), lava, "Castle_Lava"),
        part("Lava_Superior", "cylinder", (0, 0, -0.78), (2.98, 2.98, 0.10), lava_hot, "Castle_Lava_Hot"),
        part("Fortaleza_Base", "cylinder", (0, 0, 0.14), (1.74, 1.74, 1.02), brick, "Castle_Bricks"),
        part("Fortaleza_Superior", "cylinder", (0, 0, 1.45), (1.20, 1.20, 0.67), brick_dark, "Castle_Bricks_Dark"),
        part("Torre_Izquierda", "cylinder", (-1.30, 0, 0.38), (0.54, 0.54, 0.92), brick, "Castle_Bricks"),
        part("Torre_Derecha", "cylinder", (1.30, 0, 0.38), (0.54, 0.54, 0.92), brick, "Castle_Bricks"),
        part("Corona_Izquierda", "cylinder", (-1.30, 0, 1.35), (0.66, 0.66, 0.13), brick_dark, "Castle_Bricks_Dark"),
        part("Corona_Derecha", "cylinder", (1.30, 0, 1.35), (0.66, 0.66, 0.13), brick_dark, "Castle_Bricks_Dark"),
        part("Corona_Central", "cylinder", (0, 0, 2.20), (1.32, 1.32, 0.14), brick, "Castle_Bricks"),
    ]

    for i in range(10):
        angle = math.tau * i / 10
        pieces.append(part(
            f"Almena_Inferior_{i + 1:02d}", "cube",
            (math.cos(angle) * 1.60, math.sin(angle) * 1.60, 1.24),
            (0.22, 0.22, 0.30), brick, "Castle_Bricks", (0, 0, math.degrees(angle))))
    for i in range(8):
        angle = math.tau * i / 8
        pieces.append(part(
            f"Almena_Superior_{i + 1:02d}", "cube",
            (math.cos(angle) * 1.07, math.sin(angle) * 1.07, 2.36),
            (0.19, 0.19, 0.27), brick_dark, "Castle_Bricks_Dark", (0, 0, math.degrees(angle))))

    # Layered portal on the castle's front face (negative Y faces the viewer).
    for name, y, z, size, color, material in [
        ("Portal_Exterior", -1.78, -0.10, 0.86, (0.12, 0.0, 0.18, 1.0), "Castle_Portal_Outer"),
        ("Portal_Medio", -1.86, -0.10, 0.68, purple, "Castle_Portal_Middle"),
        ("Portal_Interior", -1.93, -0.10, 0.51, (0.82, 0.08, 1.0, 1.0), "Castle_Portal_Inner"),
        ("Portal_Nucleo", -2.00, -0.10, 0.31, purple_hot, "Castle_Portal_Core"),
    ]:
        pieces.append(part(name, "cube", (0, y, z), (size, 0.08, size), color, material))
    for i, (x, z) in enumerate([(-0.42, -0.48), (0.42, -0.48), (-0.42, 0.28), (0.42, 0.28), (0, -0.55), (0, 0.35)]):
        pieces.append(part(f"Luz_Portal_{i + 1}", "sphere", (x, -2.12, z), (0.09, 0.06, 0.09), purple_hot, "Castle_Portal_Core"))

    pieces.extend([
        part("Mastil", "cylinder", (0, 0, 3.50), (0.035, 0.035, 1.28), (0.12, 0.12, 0.12, 1.0), "Castle_Pole"),
        part("Bandera_1", "cube", (0.18, 0, 4.13), (0.18, 0.04, 0.18), (0.92, 0.03, 0.03, 1.0), "Castle_Flag"),
        part("Bandera_2", "cube", (0.50, 0, 4.13), (0.18, 0.04, 0.18), (0.92, 0.03, 0.03, 1.0), "Castle_Flag"),
        part("Bandera_3", "cube", (0.82, 0, 4.13), (0.18, 0.04, 0.18), (0.72, 0.01, 0.02, 1.0), "Castle_Flag_Dark"),
    ])

    for i in range(8):
        angle = math.tau * i / 8 + 0.2
        pieces.append(part(
            f"Roca_Orbital_{i + 1}", "sphere",
            (math.cos(angle) * 3.45, math.sin(angle) * 3.45, -1.30 + (i % 3) * 0.16),
            (0.22 + (i % 2) * 0.10, 0.18, 0.20),
            rock_light, "Castle_Rock"))
    return pieces


def main() -> None:
    bridge = BlenderBridge(timeout=60)
    print("Blender:", bridge.call("ping"))
    result = bridge.call("create_model", components=castle_planet_components(), clear_first=True)
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    blend_path = ASSET_DIR / "planeta_castillo_lava.blend"
    obj_path = ASSET_DIR / "planeta_castillo_lava.obj"
    bridge.call("save_blend", path=str(blend_path))
    bridge.call("export_obj", path=str(obj_path), selected_only=False)
    print(f"Creadas {result['created_count']} piezas")
    print("BLEND:", blend_path)
    print("OBJ:", obj_path)


if __name__ == "__main__":
    main()
