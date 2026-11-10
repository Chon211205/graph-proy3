"""Create the Time Planet four-star Dragon Ball through the Blender MCP add-on."""

from __future__ import annotations

import math
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from blender_mcp.bridge import BlenderBridge


ASSET_DIR = ROOT / "assets" / "levels" / "time_planet"


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


def dragon_ball_components():
    amber = (1.0, 0.31, 0.025, 1.0)
    gold = (1.0, 0.60, 0.04, 1.0)
    glow = (1.0, 0.82, 0.20, 1.0)
    red = (0.78, 0.015, 0.02, 1.0)
    deep_red = (0.36, 0.005, 0.01, 1.0)
    pieces = [
        part("Esfera_Cristal_Exterior", "sphere", (0, 0, 0), (2.65, 2.65, 2.65), amber, "Cristal_Naranja"),
        part("Esfera_Nucleo_Dorado", "sphere", (0, 0.12, 0), (2.35, 2.35, 2.35), gold, "Nucleo_Dorado"),
        part("Halo_Ecuatorial", "torus", (0, 0, 0), (2.28, 2.28, 0.14), glow, "Halo_Dragon"),
        part("Reflejo_Superior", "sphere", (-0.72, -2.23, 1.12), (0.38, 0.10, 0.62), glow, "Reflejo_Dorado", (0, 0, -28)),
    ]

    star_centers = [(-0.72, 0.78), (0.72, 0.78), (-0.72, -0.72), (0.72, -0.72)]
    for star_index, (cx, cz) in enumerate(star_centers, 1):
        pieces.append(
            part(
                f"Estrella_{star_index}_Centro",
                "sphere",
                (cx, -2.54, cz),
                (0.23, 0.09, 0.23),
                red,
                "Estrella_Roja",
            )
        )
        for point_index in range(5):
            angle = math.radians(90 + point_index * 72)
            pieces.append(
                part(
                    f"Estrella_{star_index}_Punta_{point_index + 1}",
                    "sphere",
                    (cx + math.cos(angle) * 0.28, -2.52, cz + math.sin(angle) * 0.28),
                    (0.13, 0.07, 0.27),
                    red if point_index % 2 == 0 else deep_red,
                    "Estrella_Roja" if point_index % 2 == 0 else "Estrella_Sombra",
                    (0, 0, 90 - math.degrees(angle)),
                )
            )
    return pieces


def main() -> None:
    bridge = BlenderBridge(timeout=60)
    print("Blender:", bridge.call("ping"))
    result = bridge.call("create_model", components=dragon_ball_components(), clear_first=True)
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    blend_path = ASSET_DIR / "esfera_dragon_4_estrellas.blend"
    obj_path = ASSET_DIR / "esfera_dragon_4_estrellas.obj"
    bridge.call("save_blend", path=str(blend_path))
    bridge.call("export_obj", path=str(obj_path), selected_only=False)
    print(f"Creadas {result['created_count']} piezas")
    print("BLEND:", blend_path)
    print("OBJ:", obj_path)


if __name__ == "__main__":
    main()
