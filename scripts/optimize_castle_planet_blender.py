"""Create optimized close and medium LODs for the Lava Castle planet."""

from __future__ import annotations

from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets" / "planets"
SOURCE = ASSETS / "planeta_castillo_lava.blend"


def decimate_scene(ratio: float, delete_alternate_rocks: bool = False):
    if delete_alternate_rocks:
        for obj in list(bpy.data.objects):
            if obj.name.startswith("Roca_Orbital_"):
                suffix = obj.name.rsplit("_", 1)[-1]
                if suffix.isdigit() and int(suffix) % 2 == 0:
                    bpy.data.objects.remove(obj, do_unlink=True)

    for obj in list(bpy.context.scene.objects):
        if obj.type != "MESH" or len(obj.data.polygons) < 80:
            continue
        modifier = obj.modifiers.new("Game LOD decimation", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = ratio
        modifier.use_collapse_triangulate = True
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.modifier_apply(modifier=modifier.name)
        obj.select_set(False)
        for polygon in obj.data.polygons:
            polygon.use_smooth = True


def export(path: Path):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.wm.obj_export(
        filepath=str(path),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )


def main():
    bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
    decimate_scene(0.62)
    bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS / "planeta_castillo_lava_optimizado.blend"))
    export(ASSETS / "planeta_castillo_lava_optimizado.obj")

    # Continue reducing the already optimized scene for the mid-distance LOD.
    decimate_scene(0.32, delete_alternate_rocks=True)
    export(ASSETS / "planeta_castillo_lava_optimizado_medium.obj")
    print("Optimized Lava Castle planet LODs created")


if __name__ == "__main__":
    main()
