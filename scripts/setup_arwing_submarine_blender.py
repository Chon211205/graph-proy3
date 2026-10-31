"""Create a Blender file with PBR materials for arwing2_submarino.obj."""

from __future__ import annotations

from pathlib import Path
import sys

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
import setup_interceptor_blender as setup


ROOT = Path(__file__).resolve().parents[1]
setup.OBJ_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2_submarino.obj"
setup.BLEND_PATH = ROOT / "assets" / "vehicles" / "arwing" / "arwing2_submarino.blend"


def main() -> None:
    # This wrapper is intended for a fresh/background Blender scene.
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    setup.main()


if __name__ == "__main__":
    main()
