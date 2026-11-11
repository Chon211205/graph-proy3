"""Build runtime-optimized meshes for Lava Castle Part 1."""

from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[1]


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for block in bpy.data.meshes:
        if block.users == 0:
            bpy.data.meshes.remove(block)
    for block in list(bpy.data.materials):
        bpy.data.materials.remove(block)


def optimize(source, destination, default_ratio, protected_groups=()):
    clear_scene()
    bpy.ops.wm.obj_import(filepath=str(source), forward_axis="NEGATIVE_Z", up_axis="Y")
    for obj in list(bpy.context.scene.objects):
        if obj.type != "MESH" or len(obj.data.polygons) < 32:
            continue
        ratio = 0.88 if any(group in obj.name for group in protected_groups) else default_ratio
        modifier = obj.modifiers.new("Runtime LOD", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = ratio
        modifier.use_collapse_triangulate = True
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.modifier_apply(modifier=modifier.name)
        obj.select_set(False)

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.wm.obj_export(
        filepath=str(destination),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )
    print("Optimized", source.name, "->", destination.name)


def main():
    arena = ROOT / "assets" / "levels" / "lava_castle"
    boss = ROOT / "assets" / "bosses" / "lava_guardian"
    optimize(
        arena / "lava_castle_environment.obj",
        arena / "lava_castle_environment_optimized.obj",
        0.56,
        ("Lava_Basin", "Castle_Main_Keep"),
    )
    optimize(
        boss / "guardian_lava_gigante.obj",
        boss / "guardian_lava_gigante_optimized.obj",
        0.66,
        ("Core_Inner", "Core_Outer", "Eye"),
    )


if __name__ == "__main__":
    main()
