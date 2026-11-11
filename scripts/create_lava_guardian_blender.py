"""Create the gigantic robotic Lava Guardian boss for Castle Part 1."""

from __future__ import annotations

import math
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "bosses" / "lava_guardian"


def material(name, color, metallic=0.0, roughness=0.4, emission=None):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1.0)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if emission:
        shader.inputs["Emission Color"].default_value = (*emission, 1.0)
        shader.inputs["Emission Strength"].default_value = 7.0
    return mat


def finish(obj, name, mat, smooth=True):
    obj.name = name
    obj.data.materials.append(mat)
    if smooth:
        for polygon in obj.data.polygons:
            polygon.use_smooth = True
    return obj


def cube(name, location, scale, mat, rotation=(0, 0, 0), bevel=0.0):
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=location, rotation=rotation)
    obj = finish(bpy.context.object, name, mat, False)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        mod = obj.modifiers.new("Armored bevel", "BEVEL")
        mod.width = bevel
        mod.segments = 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def sphere(name, location, scale, mat, segments=20, rings=10):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=rings, radius=1.0, location=location
    )
    obj = finish(bpy.context.object, name, mat)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return obj


def cylinder(name, location, radius, depth, mat, rotation=(0, 0, 0), vertices=12):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation
    )
    return finish(bpy.context.object, name, mat)


def torus(name, location, major, minor, mat, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major,
        minor_radius=minor,
        major_segments=24,
        minor_segments=8,
        location=location,
        rotation=rotation,
    )
    return finish(bpy.context.object, name, mat)


def cone_between(name, start, end, radius, mat, vertices=8):
    start, end = Vector(start), Vector(end)
    direction = end - start
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius,
        radius2=0.05,
        depth=direction.length,
        location=(start + end) * 0.5,
    )
    obj = finish(bpy.context.object, name, mat, False)
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0, 0, 1)).rotation_difference(direction)
    return obj


def build_guardian(mats):
    armor, armor_dark, metal, lava, lava_hot, core, eye = mats
    # Central body: about 36 units tall and 64 units across with wings.
    sphere("Guardian_Torso", (0, 0, 2), (11.5, 6.8, 14.0), armor, 24, 12)
    cube("Guardian_Chest_Plate", (0, -6.0, 3.5), (9.2, 1.8, 8.5), armor_dark, bevel=1.2)
    cube("Guardian_Abdomen", (0, -1.0, -10.5), (7.2, 5.2, 5.0), armor, bevel=1.0)

    # Multi-layer weak core faces the player (negative Blender Y).
    torus("Guardian_Core_Frame", (0, -8.2, 3.5), 5.2, 1.0, metal, (math.pi / 2, 0, 0))
    sphere("Guardian_Core_Outer", (0, -8.4, 3.5), (4.4, 1.5, 4.4), lava, 20, 10)
    sphere("Guardian_Core_Inner", (0, -9.3, 3.5), (2.5, 1.0, 2.5), core, 18, 9)
    torus("Guardian_Core_Energy_Ring", (0, -9.6, 3.5), 3.2, 0.35, lava_hot, (math.pi / 2, 0, 0))

    # Demon head, glowing eyes, jaw and crown horns.
    sphere("Guardian_Head", (0, -0.5, 17.2), (7.5, 5.6, 6.5), armor_dark, 20, 10)
    cube("Guardian_Face", (0, -5.3, 16.5), (6.1, 1.2, 4.1), armor, bevel=0.8)
    for side in (-1, 1):
        sphere(f"Guardian_Eye_{side}", (side * 2.8, -6.7, 18.1), (1.15, 0.55, 0.75), eye, 12, 6)
        cone_between(
            f"Guardian_Main_Horn_{side}",
            (side * 4.8, 0, 20.5),
            (side * 12.5, 1.5, 31.0),
            2.6,
            metal,
            10,
        )
        cone_between(
            f"Guardian_Cheek_Horn_{side}",
            (side * 6.0, -1.0, 15.5),
            (side * 11.5, -3.0, 12.0),
            1.8,
            armor,
            8,
        )
    cube("Guardian_Upper_Jaw", (0, -6.8, 13.1), (5.3, 1.2, 1.5), armor_dark, bevel=0.5)
    for tooth in range(7):
        x = (tooth - 3) * 1.45
        cone_between(f"Guardian_Tooth_{tooth}", (x, -8.0, 12.8), (x, -8.2, 10.5), 0.55, lava_hot, 6)

    # Massive articulated shoulders, arms and claws.
    for side in (-1, 1):
        sphere(f"Guardian_Shoulder_{side}", (side * 13.0, 0, 6.0), (6.5, 6.2, 6.5), armor, 16, 8)
        cylinder(
            f"Guardian_Upper_Arm_{side}",
            (side * 18.0, 0, -1.0),
            3.6,
            14.0,
            armor_dark,
            (0, side * math.radians(43), 0),
            12,
        )
        sphere(f"Guardian_Fist_{side}", (side * 23.0, -1.0, -8.0), (5.0, 4.8, 4.6), metal, 16, 8)
        for claw in range(3):
            cone_between(
                f"Guardian_Claw_{side}_{claw}",
                (side * (24.0 + claw * 0.5), -4.0 - claw * 1.1, -9.0 + claw * 1.2),
                (side * (30.0 + claw * 1.1), -7.5 - claw, -12.0 + claw * 0.8),
                1.2,
                lava_hot if claw == 1 else metal,
                7,
            )

    # Mechanical wings form the huge readable silhouette.
    for side in (-1, 1):
        cube(
            f"Guardian_Wing_Root_{side}",
            (side * 13.5, 4.5, 9.0),
            (7.0, 2.2, 3.0),
            metal,
            (0, 0, side * 0.18),
            0.7,
        )
        for blade in range(4):
            root = (side * (16.0 + blade * 2.2), 5.0 + blade * 0.6, 10.0 - blade * 3.0)
            tip = (side * (31.0 + blade * 3.5), 8.0 + blade, 19.0 - blade * 8.0)
            cone_between(f"Guardian_Wing_Blade_{side}_{blade}", root, tip, 2.3 - blade * 0.25, armor_dark, 6)
            cone_between(
                f"Guardian_Wing_Lava_Vein_{side}_{blade}",
                root,
                (tip[0] * 0.94, tip[1] - 0.6, tip[2]),
                0.45,
                lava,
                6,
            )
        cylinder(
            f"Guardian_Thruster_{side}",
            (side * 12.0, 6.5, -5.0),
            3.6,
            5.5,
            metal,
            (math.pi / 2, 0, 0),
            14,
        )
        sphere(f"Guardian_Thruster_Flame_{side}", (side * 12.0, 10.0, -5.0), (2.2, 3.5, 2.2), lava_hot, 14, 7)

    # Lava cracks across chest and abdomen.
    cracks = [
        ((-8, -8.0, 8), (-3, -9.0, 5)),
        ((8, -8.0, 9), (3, -9.0, 6)),
        ((-6, -7.4, -3), (-1, -8.0, -7)),
        ((5, -7.2, -4), (1, -8.0, -9)),
        ((-2, -7.2, 11), (1, -8.0, 8)),
    ]
    for index, (start, end) in enumerate(cracks):
        cone_between(f"Guardian_Lava_Crack_{index}", start, end, 0.55, lava_hot, 6)

    # Floating armor shards and fire orbs reinforce scale and phase-two energy.
    for index in range(8):
        angle = index * math.tau / 8.0
        x, y = math.cos(angle) * 19.0, math.sin(angle) * 8.5
        z = -2.0 + (index % 4) * 6.0
        cube(
            f"Guardian_Floating_Armor_{index}",
            (x, y + 3.0, z),
            (1.8, 1.0, 3.0),
            armor,
            (angle * 0.25, angle, angle * 0.4),
            0.35,
        )
        sphere(f"Guardian_Fire_Orb_{index}", (x * 1.12, y - 2.5, z), (0.9, 0.9, 0.9), lava_hot, 10, 5)


def main():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    OUTPUT.mkdir(parents=True, exist_ok=True)

    armor = material("Castle_Rock", (0.165, 0.149, 0.188), 0.12, 0.78)
    armor_dark = material("Castle_Rock_Dark", (0.078, 0.063, 0.106), 0.10, 0.84)
    metal = material("race_rgb_82_73_91", (0.322, 0.286, 0.357), 0.92, 0.18)
    lava = material("Castle_Lava", (1.0, 0.20, 0.03), 0.08, 0.18, (1.0, 0.035, 0.0))
    lava_hot = material("Castle_Lava_Hot", (1.0, 0.682, 0.094), 0.04, 0.14, (1.0, 0.30, 0.01))
    core = material("race_rgb_255_238_130", (1.0, 0.933, 0.510), 0.05, 0.10, (1.0, 0.72, 0.18))
    eye = material("race_rgb_255_30_18", (1.0, 0.118, 0.071), 0.12, 0.10, (1.0, 0.01, 0.005))
    build_guardian((armor, armor_dark, metal, lava, lava_hot, core, eye))

    blend = OUTPUT / "guardian_lava_gigante.blend"
    obj = OUTPUT / "guardian_lava_gigante.obj"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.wm.obj_export(
        filepath=str(obj),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )
    print("Created gigantic Lava Guardian in", OUTPUT)


if __name__ == "__main__":
    main()
