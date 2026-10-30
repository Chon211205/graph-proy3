"""Add production-friendly hard-surface details to both Kirby vehicles."""

from __future__ import annotations

import math
from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[1]
KIRBY_DIR = ROOT / "assets" / "vehicles" / "kirby"


def material(name: str, color: tuple[float, float, float, float], metallic=0.0, roughness=0.3,
             emission: tuple[float, float, float, float] | None = None) -> bpy.types.Material:
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.diffuse_color = color
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = color
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if emission:
        shader.inputs["Emission Color"].default_value = emission
        shader.inputs["Emission Strength"].default_value = 5.0
    return mat


PALETTE = {
    "pink": lambda: material("Kirby_Smooth_Pink", (1.0, 0.35, 0.62, 1), 0.0, 0.24),
    "gold": lambda: material("Submarine_Metallic_Gold", (0.76, 0.45, 0.06, 1), 0.82, 0.18),
    "gold_hi": lambda: material("Polished_Gold_Highlights", (1.0, 0.76, 0.18, 1), 0.72, 0.12),
    "dark": lambda: material("Mechanical_Dark.003", (0.025, 0.035, 0.055, 1), 0.88, 0.19),
    "cyan": lambda: material("Cyan_Engine_Glow", (0.02, 0.72, 1.0, 1), 0.25, 0.12, (0.01, 0.65, 1.0, 1)),
    "glass": lambda: material("Submarine_Glass", (0.03, 0.18, 0.28, 0.78), 0.12, 0.08),
}


def assign(obj: bpy.types.Object, mat: bpy.types.Material) -> bpy.types.Object:
    obj.data.materials.append(mat)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True
    return obj


def uv_sphere(name: str, location, scale, mat, segments=24, rings=12):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=segments, ring_count=rings, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return assign(obj, mat)


def torus(name: str, location, major: float, minor: float, mat, rotation=(0, 0, 0), segments=48):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major,
        minor_radius=minor,
        major_segments=segments,
        minor_segments=10,
        location=location,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    return assign(obj, mat)


def cylinder(name: str, location, radius: float, depth: float, mat, rotation=(0, 0, 0), vertices=24):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation
    )
    obj = bpy.context.object
    obj.name = name
    bevel = obj.modifiers.new("Edge bevel", "BEVEL")
    bevel.width = min(radius * 0.12, 0.07)
    bevel.segments = 2
    return assign(obj, mat)


def star(name: str, location, radius: float, thickness: float, mat) -> None:
    outline = []
    for index in range(10):
        angle = math.pi / 2 + index * math.pi / 5
        r = radius if index % 2 == 0 else radius * 0.43
        outline.append((location[0] + math.cos(angle) * r, location[1] + math.sin(angle) * r))
    vertices = [(x, y, location[2] - thickness / 2) for x, y in outline]
    vertices += [(x, y, location[2] + thickness / 2) for x, y in outline]
    faces = [tuple(range(9, -1, -1)), tuple(range(10, 20))]
    for index in range(10):
        nxt = (index + 1) % 10
        faces.append((index, nxt, 10 + nxt, 10 + index))
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(vertices, [], faces)
    assign(bpy.data.objects.new(name, mesh), mat)
    bpy.context.collection.objects.link(bpy.data.objects[name])


def clear_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (bpy.data.meshes, bpy.data.materials, bpy.data.curves):
        for block in list(datablocks):
            if block.users == 0:
                datablocks.remove(block)


def import_obj(path: Path) -> list[bpy.types.Object]:
    before = set(bpy.data.objects)
    bpy.ops.wm.obj_import(filepath=str(path), forward_axis="Y", up_axis="Z")
    objects = [obj for obj in bpy.data.objects if obj not in before]
    for obj in objects:
        if obj.type == "MESH":
            for polygon in obj.data.polygons:
                polygon.use_smooth = True
    return objects


def add_ufo_details() -> None:
    gold, gold_hi, dark, cyan = PALETTE["gold"](), PALETTE["gold_hi"](), PALETTE["dark"](), PALETTE["cyan"]()
    torus("UFO_Outer_Armor_Ring", (0, 0, -0.72), 7.15, 0.14, gold, segments=64)
    torus("UFO_Cyan_Energy_Ring", (0, 0, -0.87), 6.68, 0.075, cyan, segments=64)
    torus("UFO_Upper_Trim", (0, 0, 0.22), 5.88, 0.08, gold_hi, segments=56)
    star("Kirby_Star_Emblem", (0, -0.25, 5.22), 0.82, 0.11, gold_hi)

    for index in range(16):
        angle = math.tau * index / 16
        uv_sphere(
            f"Rim_Light_{index:02d}",
            (math.cos(angle) * 7.20, math.sin(angle) * 7.20, -0.69),
            (0.14, 0.14, 0.10), cyan, 16, 8,
        )
    for side, x in (("L", -1.65), ("R", 1.65)):
        cylinder(f"Rear_Thruster_{side}", (x, 6.45, -1.05), 0.44, 0.72, dark, (math.pi / 2, 0, 0))
        cylinder(f"Rear_Core_{side}", (x, 6.83, -1.05), 0.29, 0.08, cyan, (math.pi / 2, 0, 0))
        torus(f"Thruster_Guard_{side}", (x, 6.79, -1.05), 0.48, 0.065, gold, (math.pi / 2, 0, 0), 32)


def add_submarine_details() -> None:
    gold, gold_hi, dark, cyan, glass = (
        PALETTE["gold"](), PALETTE["gold_hi"](), PALETTE["dark"](),
        PALETTE["cyan"](), PALETTE["glass"](),
    )
    torus("Submarine_Pressure_Ring", (0, 0, -0.72), 7.18, 0.16, gold, segments=64)
    torus("Submarine_Energy_Band", (0, 0, -0.92), 6.72, 0.08, cyan, segments=64)
    star("Submarine_Star_Emblem", (0, -0.20, 5.02), 0.74, 0.12, gold_hi)

    for index in range(10):
        angle = math.tau * index / 10
        uv_sphere(
            f"Pressure_Lamp_{index:02d}",
            (math.cos(angle) * 7.12, math.sin(angle) * 7.12, -0.68),
            (0.18, 0.18, 0.12), cyan, 16, 8,
        )
    for side, x in (("L", -2.15), ("R", 2.15)):
        cylinder(f"Sub_Propeller_Hub_{side}", (x, 6.65, -0.85), 0.34, 0.78, dark, (math.pi / 2, 0, 0))
        cylinder(f"Sub_Propeller_Core_{side}", (x, 7.05, -0.85), 0.22, 0.10, cyan, (math.pi / 2, 0, 0))
        torus(f"Sub_Propeller_Guard_{side}", (x, 7.02, -0.85), 0.78, 0.09, gold, (math.pi / 2, 0, 0), 40)
        for blade in range(4):
            angle = math.tau * blade / 4
            uv_sphere(
                f"Sub_Blade_{side}_{blade}",
                (x + math.cos(angle) * 0.42, 7.03, -0.85 + math.sin(angle) * 0.42),
                (0.28, 0.07, 0.13), gold_hi, 12, 6,
            )
    for side, x in (("L", -3.6), ("R", 3.6)):
        uv_sphere(f"Observation_Port_{side}", (x, -1.25, 1.0), (0.38, 0.16, 0.38), glass, 24, 12)


def export_obj(path: Path) -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.wm.obj_export(
        filepath=str(path),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        apply_modifiers=True,
        forward_axis="Y",
        up_axis="Z",
    )


def build(source_name: str, target_name: str, detail_builder) -> None:
    clear_scene()
    import_obj(KIRBY_DIR / source_name)
    detail_builder()
    export_obj(KIRBY_DIR / target_name)
    print(f"Generated {target_name}")


if __name__ == "__main__":
    build("kirby.obj", "kirby_detallado.obj", add_ufo_details)
    build("kirbysub.obj", "kirbysub_detallado.obj", add_submarine_details)
