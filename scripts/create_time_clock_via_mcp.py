"""Create the Time Planet collectible clock through the local Blender MCP add-on."""

from __future__ import annotations

import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from blender_mcp.bridge import BlenderBridge


ASSET_DIR = ROOT / "assets" / "levels" / "time_planet"


def part(
    name: str,
    primitive: str,
    location: tuple[float, float, float],
    scale: tuple[float, float, float],
    color: tuple[float, float, float, float],
    material: str,
    rotation: tuple[float, float, float] = (0.0, 0.0, 0.0),
) -> dict[str, object]:
    return {
        "name": name,
        "primitive": primitive,
        "location": location,
        "rotation": rotation,
        "scale": scale,
        "color": color,
        "material_name": material,
    }


def clock_components() -> list[dict[str, object]]:
    gold = (0.95, 0.52, 0.06, 1.0)
    dark_gold = (0.55, 0.20, 0.025, 1.0)
    cyan = (0.04, 0.65, 0.95, 1.0)
    pale = (0.78, 0.94, 1.0, 1.0)
    navy = (0.015, 0.045, 0.12, 1.0)
    white = (1.0, 0.96, 0.72, 1.0)

    pieces = [
        part("Reloj_Cuerpo", "cylinder", (0, 0, 0), (3.12, 3.12, 0.62), dark_gold, "Oro_Oscuro", (90, 0, 0)),
        part("Reloj_Esfera", "cylinder", (0, -0.78, 0), (2.68, 2.68, 0.18), pale, "Cristal_Celeste", (90, 0, 0)),
        part("Reloj_Marco", "torus", (0, -1.0, 0), (3.12, 3.12, 0.55), gold, "Oro_Brillante", (90, 0, 0)),
        part("Reloj_Aro_Energia", "torus", (0, -1.16, 0), (2.64, 2.64, 0.12), cyan, "Energia_Cian", (90, 0, 0)),
        part("Reloj_Centro", "cylinder", (0, -1.38, 0), (0.28, 0.28, 0.16), gold, "Oro_Brillante", (90, 0, 0)),
    ]

    radius = 2.15
    for index in range(12):
        angle = math.radians(index * 30)
        x = radius * math.sin(angle)
        z = radius * math.cos(angle)
        pieces.append(
            part(
                f"Marca_{index + 1:02d}",
                "cube",
                (x, -1.26, z),
                (0.105, 0.10, 0.30 if index % 3 == 0 else 0.22),
                gold if index % 3 == 0 else navy,
                "Oro_Brillante" if index % 3 == 0 else "Azul_Profundo",
                (0, index * 30, 0),
            )
        )

    def hand(name: str, angle_deg: float, length: float, width: float, color, material):
        angle = math.radians(angle_deg)
        return part(
            name,
            "cube",
            (math.sin(angle) * length / 2, -1.40, math.cos(angle) * length / 2),
            (width, 0.095, length / 2),
            color,
            material,
            (0, angle_deg, 0),
        )

    pieces.extend(
        [
            hand("Manecilla_Hora", -55, 1.30, 0.16, navy, "Azul_Profundo"),
            hand("Manecilla_Minuto", 25, 1.82, 0.10, cyan, "Energia_Cian"),
            part("Campana_Izquierda", "sphere", (-2.22, 0, 2.55), (1.10, 0.72, 0.62), gold, "Oro_Brillante", (0, 20, -18)),
            part("Campana_Derecha", "sphere", (2.22, 0, 2.55), (1.10, 0.72, 0.62), gold, "Oro_Brillante", (0, -20, 18)),
            part("Union_Izquierda", "cube", (-1.75, 0, 2.12), (0.16, 0.22, 0.62), dark_gold, "Oro_Oscuro", (0, -28, 0)),
            part("Union_Derecha", "cube", (1.75, 0, 2.12), (0.16, 0.22, 0.62), dark_gold, "Oro_Oscuro", (0, 28, 0)),
            part("Corona", "cone", (0, 0, 3.30), (0.46, 0.46, 0.55), cyan, "Energia_Cian"),
            part("Boton_Corona", "sphere", (0, 0, 3.65), (0.28, 0.28, 0.28), white, "Luz_Temporal"),
            part("Pata_Izquierda", "cube", (-1.65, 0, -2.78), (0.55, 0.48, 0.24), gold, "Oro_Brillante", (0, -18, 0)),
            part("Pata_Derecha", "cube", (1.65, 0, -2.78), (0.55, 0.48, 0.24), gold, "Oro_Brillante", (0, 18, 0)),
            part("Nucleo_Temporal", "sphere", (0, -1.58, 0), (0.13, 0.13, 0.13), white, "Luz_Temporal"),
        ]
    )
    return pieces


def main() -> None:
    bridge = BlenderBridge(timeout=60)
    print("Blender:", bridge.call("ping"))
    result = bridge.call("create_model", components=clock_components(), clear_first=True)
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    blend_path = ASSET_DIR / "reloj_tiempo.blend"
    obj_path = ASSET_DIR / "reloj_tiempo.obj"
    bridge.call("save_blend", path=str(blend_path))
    bridge.call("export_obj", path=str(obj_path), selected_only=False)
    scene = bridge.call("scene_info")
    print(f"Creadas {result['created_count']} piezas; escena con {scene['object_count']} objetos")
    print("BLEND:", blend_path)
    print("OBJ:", obj_path)


if __name__ == "__main__":
    main()
