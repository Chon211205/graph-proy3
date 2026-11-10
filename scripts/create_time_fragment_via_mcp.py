"""Create the Time Planet energy fragment through the local Blender MCP add-on."""

from __future__ import annotations

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


def fragment_components():
    cyan = (0.03, 0.82, 1.0, 1.0)
    pale = (0.62, 0.96, 1.0, 1.0)
    violet = (0.47, 0.12, 0.82, 1.0)
    deep = (0.08, 0.03, 0.24, 1.0)
    white = (1.0, 0.96, 0.74, 1.0)
    return [
        part("Fragmento_Nucleo", "sphere", (0, 0, 0.15), (0.82, 0.82, 0.82), white, "Luz_Fragmento"),
        part("Cristal_Central_Superior", "cone", (0, 0, 1.35), (1.10, 1.10, 1.75), cyan, "Cristal_Energia"),
        part("Cristal_Central_Inferior", "cone", (0, 0, -0.75), (1.10, 1.10, 0.78), violet, "Cristal_Violeta", (180, 0, 0)),
        part("Cristal_Interior", "cone", (0, -0.35, 1.25), (0.48, 0.48, 1.45), pale, "Cristal_Claro"),
        part("Fragmento_Izquierdo", "cone", (-1.18, 0.05, 0.45), (0.44, 0.44, 1.18), cyan, "Cristal_Energia", (0, -24, -10)),
        part("Fragmento_Derecho", "cone", (1.18, 0.05, 0.35), (0.40, 0.40, 1.05), pale, "Cristal_Claro", (0, 27, 12)),
        part("Fragmento_Trasero", "cone", (0.18, 0.88, 0.22), (0.42, 0.42, 0.95), violet, "Cristal_Violeta", (-22, 0, 8)),
        part("Aro_Temporal_Exterior", "torus", (0, 0, -0.85), (1.72, 1.72, 0.15), cyan, "Aro_Energia"),
        part("Aro_Temporal_Interior", "torus", (0, 0, -0.83), (1.28, 1.28, 0.08), violet, "Cristal_Violeta"),
        part("Orbe_Izquierdo", "sphere", (-1.72, 0, -0.82), (0.18, 0.18, 0.18), white, "Luz_Fragmento"),
        part("Orbe_Derecho", "sphere", (1.72, 0, -0.82), (0.18, 0.18, 0.18), white, "Luz_Fragmento"),
        part("Base_Sombra", "cylinder", (0, 0, -1.05), (0.68, 0.68, 0.10), deep, "Energia_Profunda"),
    ]


def main() -> None:
    bridge = BlenderBridge(timeout=60)
    print("Blender:", bridge.call("ping"))
    result = bridge.call("create_model", components=fragment_components(), clear_first=True)
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    blend_path = ASSET_DIR / "fragmento_energia.blend"
    obj_path = ASSET_DIR / "fragmento_energia.obj"
    bridge.call("save_blend", path=str(blend_path))
    bridge.call("export_obj", path=str(obj_path), selected_only=False)
    print(f"Creadas {result['created_count']} piezas")
    print("BLEND:", blend_path)
    print("OBJ:", obj_path)


if __name__ == "__main__":
    main()
