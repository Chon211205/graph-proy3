"""Build the optimized exterior arena for Lava Castle Part 1.

Run with Blender 4.x:
    blender --background --python scripts/create_lava_castle_arena_blender.py
"""

from __future__ import annotations

import math
from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "levels" / "lava_castle"


def material(name, color, metallic=0.0, roughness=0.5, emission=None):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1.0)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if emission:
        shader.inputs["Emission Color"].default_value = (*emission, 1.0)
        shader.inputs["Emission Strength"].default_value = 5.0
    return mat


def finish(obj, name, mat, collection, smooth=False):
    obj.name = name
    obj.data.materials.append(mat)
    for old in list(obj.users_collection):
        old.objects.unlink(obj)
    collection.objects.link(obj)
    if smooth:
        for polygon in obj.data.polygons:
            polygon.use_smooth = True
    return obj


def cube(name, location, scale, mat, collection, bevel=0.0, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=location, rotation=rotation)
    obj = finish(bpy.context.object, name, mat, collection)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        mod = obj.modifiers.new("Weathered edges", "BEVEL")
        mod.width = bevel
        mod.segments = 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def cylinder(name, location, radius, depth, mat, collection, vertices=12):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location
    )
    return finish(bpy.context.object, name, mat, collection, True)


def cone(name, location, radius, depth, mat, collection, vertices=12):
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices, radius1=radius, radius2=0.0, depth=depth, location=location
    )
    return finish(bpy.context.object, name, mat, collection)


def torus(name, location, major, minor, mat, collection, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major,
        minor_radius=minor,
        major_segments=20,
        minor_segments=6,
        location=location,
        rotation=rotation,
    )
    return finish(bpy.context.object, name, mat, collection, True)


def add_battlements(prefix, y, z, half_width, mat, collection):
    count = max(3, int(half_width / 5))
    for index in range(-count, count + 1):
        x = index * half_width / count
        cube(
            f"{prefix}_Battlement_{index + count:02}",
            (x, y, z),
            (2.0, 2.2, 3.0),
            mat,
            collection,
            0.25,
        )


def build_environment(collection, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, window, metal = mats

    # Layered lava basin and safe combat islands.
    cube("Lava_Basin", (0, 75, -2.0), (126, 142, 2.0), lava, collection)
    cube("Lava_Hot_Core", (0, 78, -1.4), (112, 132, 0.35), lava_hot, collection)
    for index, (x, y, sx, sy, angle) in enumerate(
        [
            (-84, 45, 25, 44, -0.10),
            (-38, 69, 27, 50, 0.08),
            (10, 48, 24, 46, -0.04),
            (58, 73, 25, 48, 0.10),
            (96, 43, 20, 38, -0.12),
        ]
    ):
        cube(
            f"Arena_Island_{index}",
            (x, y, 3.0),
            (sx, sy, 4.0),
            rock,
            collection,
            2.2,
            (0, 0, angle),
        )
        # Bright cracks sit just above the rock and remain readable at distance.
        for crack in range(3):
            cube(
                f"Island_{index}_Lava_Crack_{crack}",
                (x - sx * 0.55 + crack * sx * 0.55, y, 7.15),
                (0.75, sy * 0.72, 0.12),
                lava_hot,
                collection,
                0.15,
                (0, 0, angle + (crack - 1) * 0.08),
            )

    # Castle body at engine Z=-202 (Blender Y=202).
    cube("Castle_Main_Keep", (0, 202, 45), (82, 12, 45), brick_dark, collection, 1.2)
    cube("Castle_Front_Facade", (0, 188, 38), (72, 4, 38), brick, collection, 0.8)
    add_battlements("Main_Keep", 187, 80, 72, brick_dark, collection)

    for index, x in enumerate((-72, -40, 40, 72)):
        cylinder(f"Castle_Tower_{index}", (x, 198, 66), 13, 72, brick_dark, collection, 12)
        cone(f"Tower_Roof_{index}", (x, 198, 111), 18, 27, metal, collection, 12)
        torus(f"Tower_Collar_{index}", (x, 198, 91), 13.2, 1.2, brick, collection)
        for floor in range(3):
            cube(
                f"Tower_{index}_Window_{floor}",
                (x, 184.4, 43 + floor * 20),
                (3.2, 0.7, 6.0),
                window,
                collection,
                0.55,
            )
        for side in (-1, 1):
            cube(
                f"Tower_{index}_Spike_{side}",
                (x + side * 14.5, 198, 91),
                (5.5, 2.2, 2.2),
                metal,
                collection,
                0.35,
                (0, side * 0.35, 0),
            )

    # Door frame, chains, glowing runes and lava falls flank the animated door.
    for side in (-1, 1):
        cube(
            f"Door_Pillar_{side}",
            (side * 20, 188, 36),
            (6, 5, 34),
            brick_dark,
            collection,
            0.8,
        )
        cube(
            f"Lava_Fall_{side}",
            (side * 31, 186.5, 35),
            (3.2, 1.0, 31),
            lava_hot,
            collection,
            0.5,
        )
        for link in range(7):
            torus(
                f"Door_Chain_{side}_{link}",
                (side * 13, 184, 18 + link * 7),
                2.4,
                0.55,
                metal,
                collection,
                (math.pi / 2, 0, 0),
            )
    cube("Door_Lintel", (0, 188, 66), (27, 5, 6), brick_dark, collection, 0.8)
    torus("Door_Rune", (0, 182.8, 53), 10, 1.1, lava_hot, collection, (math.pi / 2, 0, 0))

    # Volcanic horizon silhouettes and lava vents.
    for index, (x, y, radius, height) in enumerate(
        [(-112, 238, 38, 80), (-74, 260, 34, 68), (88, 256, 42, 88), (122, 232, 32, 65)]
    ):
        cone(f"Horizon_Volcano_{index}", (x, y, height * 0.42), radius, height, rock_dark, collection, 10)
        cylinder(f"Volcano_Vent_{index}", (x, y - 1, height * 0.82), radius * 0.20, 2.0, lava_hot, collection, 10)

    # Foreground obstacles provide readable cover without closing the flight plane.
    for index, (x, y, z, size) in enumerate(
        [(-92, 112, 18, 12), (-55, 132, 27, 15), (72, 119, 23, 13), (101, 145, 34, 17)]
    ):
        cone(f"Volcanic_Spire_{index}", (x, y, z), size, size * 2.6, rock, collection, 7)


def build_door(collection, mats):
    _, _, brick, brick_dark, lava, lava_hot, _, metal = mats
    cube("Animated_Castle_Door", (0, 191, 34), (15, 3, 25), brick_dark, collection, 1.0)
    for side in (-1, 1):
        cube(
            f"Door_Metal_Rib_{side}",
            (side * 8.5, 187.8, 34),
            (1.1, 0.7, 23),
            metal,
            collection,
            0.25,
        )
    for row in range(4):
        cube(
            f"Door_Lava_Rune_{row}",
            (0, 187.1, 18 + row * 10),
            (5.2 - row * 0.45, 0.35, 0.75),
            lava_hot if row % 2 else lava,
            collection,
            0.2,
        )


def export_collection(collection, path):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in collection.objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = next(iter(collection.objects))
    bpy.ops.wm.obj_export(
        filepath=str(path),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )


def main():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    environment = bpy.data.collections.new("Lava Castle Environment")
    door = bpy.data.collections.new("Lava Castle Door")
    bpy.context.scene.collection.children.link(environment)
    bpy.context.scene.collection.children.link(door)

    rock = material("Castle_Rock", (0.15, 0.11, 0.12), 0.12, 0.86)
    rock_dark = material("Castle_Rock_Dark", (0.07, 0.045, 0.065), 0.20, 0.82)
    brick = material("Castle_Bricks", (0.23, 0.19, 0.25), 0.28, 0.70)
    brick_dark = material("Castle_Bricks_Dark", (0.095, 0.065, 0.11), 0.35, 0.64)
    lava = material("Castle_Lava", (0.95, 0.05, 0.008), 0.05, 0.24, (1.0, 0.025, 0.0))
    lava_hot = material("Castle_Lava_Hot", (1.0, 0.42, 0.015), 0.02, 0.18, (1.0, 0.20, 0.005))
    window = material("Castle_Portal_Core", (0.95, 0.09, 0.025), 0.10, 0.16, (1.0, 0.035, 0.005))
    metal = material("Castle_Pole", (0.055, 0.045, 0.07), 0.88, 0.24)
    mats = (rock, rock_dark, brick, brick_dark, lava, lava_hot, window, metal)

    build_environment(environment, mats)
    build_door(door, mats)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUTPUT / "lava_castle_arena.blend"))
    export_collection(environment, OUTPUT / "lava_castle_environment.obj")
    export_collection(door, OUTPUT / "lava_castle_door.obj")
    print("Created Lava Castle arena in", OUTPUT)


if __name__ == "__main__":
    main()
