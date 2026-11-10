"""Create the three clockwork sentinels used by the Time Planet.

Run with Blender 4.x:
    blender --background --python scripts/create_time_enemies_blender.py
"""

from __future__ import annotations

import math
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "enemies" / "time_planet"


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
        shader.inputs["Emission Strength"].default_value = 5.0
    return mat


def finish(obj, name, mat, smooth=True):
    obj.name = name
    obj.data.materials.append(mat)
    if smooth:
        for polygon in obj.data.polygons:
            polygon.use_smooth = True
    return obj


def uv(name, location, scale, mat, segments=16, rings=8):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=rings, radius=1.0, location=location
    )
    obj = finish(bpy.context.object, name, mat)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return obj


def cube(name, location, scale, mat, rotation=(0, 0, 0), bevel=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rotation)
    obj = finish(bpy.context.object, name, mat, False)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        mod = obj.modifiers.new("Bisel", "BEVEL")
        mod.width = bevel
        mod.segments = 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def cylinder(name, location, radius, depth, mat, rotation=(0, 0, 0), vertices=16):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation
    )
    return finish(bpy.context.object, name, mat)


def torus(name, location, major, minor, mat, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major,
        minor_radius=minor,
        major_segments=20,
        minor_segments=6,
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
        radius2=0.025,
        depth=direction.length,
        location=(start + end) * 0.5,
    )
    obj = finish(bpy.context.object, name, mat, False)
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0, 0, 1)).rotation_difference(direction)
    return obj


def create_chronoguard(m):
    brass, dark, violet, cyan, ivory = m
    p = "CHRONOGUARD"
    cylinder(p + "_Body", (0, 0, 0.2), 1.15, 2.1, dark, vertices=12)
    torus(p + "_Clock_Rim", (0, -1.08, 0.45), 0.78, 0.13, brass, (math.pi / 2, 0, 0))
    cylinder(p + "_Clock_Face", (0, -1.04, 0.45), 0.72, 0.10, ivory, (math.pi / 2, 0, 0), 20)
    for angle in range(0, 360, 45):
        a = math.radians(angle)
        uv(p + f"_Hour_{angle}", (math.sin(a) * 0.57, -1.13, 0.45 + math.cos(a) * 0.57), (0.055, 0.04, 0.055), violet, 8, 4)
    cube(p + "_Hand_Minute", (0.18, -1.16, 0.63), (0.05, 0.035, 0.33), violet, (0, 0, -0.55), 0.02)
    cube(p + "_Hand_Hour", (-0.17, -1.17, 0.48), (0.04, 0.035, 0.23), dark, (0, 0, 0.85), 0.02)
    uv(p + "_Eye", (0, -1.20, 1.55), (0.33, 0.12, 0.22), cyan, 12, 6)
    for side in (-1, 1):
        cylinder(p + f"_Shoulder_{side}", (side * 1.27, 0, 0.62), 0.33, 0.52, brass, (0, math.pi / 2, 0), 12)
        cone_between(p + f"_Arm_{side}", (side * 1.45, 0, 0.55), (side * 2.05, -0.18, -0.35), 0.24, dark)
        cone_between(p + f"_Leg_{side}", (side * 0.52, 0, -0.75), (side * 0.78, 0.05, -1.75), 0.28, brass)
        cube(p + f"_Foot_{side}", (side * 0.80, -0.25, -1.82), (0.44, 0.62, 0.18), dark, bevel=0.10)
    torus(p + "_Halo", (0, 0.15, 1.75), 0.68, 0.08, violet, (math.pi / 2, 0, 0))


def create_pendulum_reaper(m):
    brass, dark, violet, cyan, ivory = m
    p = "PENDULUM_REAPER"
    uv(p + "_Core", (0, 0, 0.35), (0.92, 0.62, 1.12), dark, 16, 8)
    torus(p + "_Cage", (0, 0, 0.35), 1.18, 0.12, brass, (math.pi / 2, 0, 0))
    torus(p + "_Gyro", (0, 0, 0.35), 0.92, 0.08, violet, (0, math.pi / 2, 0))
    uv(p + "_Eye", (0, -0.64, 0.62), (0.34, 0.12, 0.22), cyan, 12, 6)
    cylinder(p + "_Pendulum_Rod", (0, 0, -1.05), 0.09, 1.75, brass, vertices=10)
    uv(p + "_Pendulum_Bob", (0, 0, -2.0), (0.56, 0.42, 0.68), violet, 14, 7)
    for side in (-1, 1):
        cone_between(p + f"_Blade_Arm_{side}", (side * 0.78, 0, 0.55), (side * 1.75, -0.05, 0.05), 0.18, brass)
        cone_between(p + f"_Blade_{side}", (side * 1.48, -0.05, -0.05), (side * 2.35, -0.10, -0.82), 0.34, ivory, 4)
        uv(p + f"_Thruster_{side}", (side * 0.68, 0.35, -0.55), (0.18, 0.18, 0.30), cyan, 10, 5)


def create_hourglass_wraith(m):
    brass, dark, violet, cyan, ivory = m
    p = "HOURGLASS_WRAITH"
    torus(p + "_Top_Rim", (0, 0, 1.15), 0.86, 0.12, brass)
    torus(p + "_Bottom_Rim", (0, 0, -1.15), 0.86, 0.12, brass)
    for side in (-1, 1):
        for z in (-0.9, 0.9):
            cone_between(p + f"_Glass_{side}_{z}", (side * 0.72, 0, z), (side * 0.10, 0, 0), 0.34, violet, 12)
    uv(p + "_Sand_Upper", (0, -0.08, 0.55), (0.58, 0.45, 0.48), ivory, 14, 7)
    cone_between(p + "_Sand_Stream", (0, -0.08, 0.45), (0, -0.08, -0.65), 0.08, cyan, 10)
    uv(p + "_Sand_Lower", (0, -0.08, -0.78), (0.64, 0.48, 0.30), cyan, 14, 7)
    uv(p + "_Eye", (0, -0.58, 0.02), (0.30, 0.12, 0.20), cyan, 12, 6)
    for index in range(4):
        angle = index * math.tau / 4 + math.pi / 4
        x, y = math.cos(angle) * 0.68, math.sin(angle) * 0.68
        cylinder(p + f"_Pillar_{index}", (x, y, 0), 0.10, 2.18, dark, vertices=8)
    for side in (-1, 1):
        cone_between(p + f"_Horn_{side}", (side * 0.55, 0, 1.35), (side * 1.28, 0.05, 1.92), 0.22, violet)
        cone_between(p + f"_Claw_{side}", (side * 0.72, 0, -1.15), (side * 1.55, -0.15, -1.62), 0.24, dark)


def export_enemy(filename, creator, mats):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    creator(mats)
    blend = OUTPUT / f"{filename}.blend"
    obj = OUTPUT / f"{filename}.obj"
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


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    brass = material("time_rgb_190_126_49", (0.745, 0.494, 0.192), 0.78, 0.24)
    dark = material("time_rgb_39_29_65", (0.153, 0.114, 0.255), 0.58, 0.30)
    violet = material("time_rgb_170_52_190", (0.667, 0.204, 0.745), 0.28, 0.20, (0.55, 0.03, 0.72))
    cyan = material("time_rgb_55_232_244", (0.216, 0.910, 0.957), 0.18, 0.16, (0.03, 0.82, 1.0))
    ivory = material("time_rgb_246_218_167", (0.965, 0.855, 0.655), 0.10, 0.30)
    mats = (brass, dark, violet, cyan, ivory)
    export_enemy("chronoguard", create_chronoguard, mats)
    export_enemy("pendulum_reaper", create_pendulum_reaper, mats)
    export_enemy("hourglass_wraith", create_hourglass_wraith, mats)
    print("Created Time Planet enemies in", OUTPUT)


if __name__ == "__main__":
    main()
