"""Build the detailed racing planet used by the home galaxy.

Run with Blender 4.x:
    blender --background --python scripts/create_racing_planet_blender.py
"""

from __future__ import annotations

import math
from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "planets"


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


def smooth(obj, materials):
    for mat in materials:
        obj.data.materials.append(mat)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True
    return obj


def create_surface(green, light_green, dark_green):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=5, radius=27.3)
    obj = smooth(bpy.context.object, (green, light_green, dark_green))
    obj.name = "Race_Planet_Terrain"
    for vertex in obj.data.vertices:
        direction = vertex.co.normalized()
        terrain = (
            math.sin(direction.x * 8.0 + direction.z * 3.0) * 0.30
            + math.sin(direction.y * 13.0 - direction.x * 4.0) * 0.18
            + math.sin((direction.x + direction.z) * 21.0) * 0.08
        )
        # Flatten the equator beneath the racing belt.
        equator_fade = min(1.0, abs(direction.y) * 5.0)
        vertex.co = direction * (27.3 + terrain * equator_fade)
    for index, polygon in enumerate(obj.data.polygons):
        value = (index * 37 + index // 17) % 11
        polygon.material_index = 1 if value < 3 else (2 if value == 10 else 0)
    return obj


def create_track(materials):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=28.45,
        minor_radius=2.15,
        major_segments=96,
        minor_segments=12,
        rotation=(math.pi / 2, 0.0, 0.0),
    )
    obj = smooth(bpy.context.object, materials)
    obj.name = "Race_Equatorial_Checkered_Track"
    for polygon in obj.data.polygons:
        center = polygon.center
        angle = math.atan2(center.z, center.x) % (2.0 * math.pi)
        sector = int(angle / (2.0 * math.pi) * 48)
        side = 0 if center.y >= 0.0 else 1
        if abs(center.y) < 0.55:
            polygon.material_index = 0  # asphalt center
        else:
            polygon.material_index = 1 + ((sector + side) & 1)
    return obj


def torus(name, major, minor, mat, rotation=(math.pi / 2, 0.0, 0.0), segments=96):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major,
        minor_radius=minor,
        major_segments=segments,
        minor_segments=8,
        rotation=rotation,
    )
    obj = smooth(bpy.context.object, (mat,))
    obj.name = name
    return obj


def create_boost_lights(mat):
    for index in range(16):
        angle = index * math.tau / 16.0
        radius = 30.65
        location = (math.cos(angle) * radius, 0.0, math.sin(angle) * radius)
        bpy.ops.mesh.primitive_uv_sphere_add(segments=12, ring_count=6, radius=0.32, location=location)
        obj = smooth(bpy.context.object, (mat,))
        obj.name = f"Race_Boost_Light_{index:02}"


def create_polar_beacon(mat, dark):
    for sign in (-1.0, 1.0):
        bpy.ops.mesh.primitive_cylinder_add(
            vertices=18,
            radius=0.72,
            depth=3.8,
            location=(0.0, sign * 28.5, 0.0),
            rotation=(math.pi / 2, 0.0, 0.0),
        )
        mast = smooth(bpy.context.object, (dark,))
        mast.name = "Race_Polar_Mast_N" if sign > 0 else "Race_Polar_Mast_S"
        bpy.ops.mesh.primitive_uv_sphere_add(
            segments=18,
            ring_count=9,
            radius=1.05,
            location=(0.0, sign * 30.5, 0.0),
        )
        beacon = smooth(bpy.context.object, (mat,))
        beacon.name = "Race_Polar_Beacon_N" if sign > 0 else "Race_Polar_Beacon_S"


def main():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    OUTPUT.mkdir(parents=True, exist_ok=True)

    green = material("race_rgb_44_133_88", (0.17255, 0.52157, 0.34510), roughness=0.76)
    green_hi = material("race_rgb_58_160_107", (0.22745, 0.62745, 0.41961), roughness=0.68)
    green_dark = material("race_rgb_25_92_61", (0.098, 0.361, 0.239), roughness=0.82)
    asphalt = material("race_rgb_60_64_73", (0.23529, 0.25098, 0.28627), metallic=0.22, roughness=0.52)
    white = material("race_rgb_245_246_239", (0.96078, 0.96471, 0.93725), metallic=0.12, roughness=0.32)
    black = material("race_rgb_30_34_42", (0.11765, 0.13333, 0.16471), metallic=0.32, roughness=0.28)
    yellow = material("race_rgb_245_208_42", (0.96078, 0.81569, 0.16471), metallic=0.36, roughness=0.24)
    cyan = material("race_rgb_54_220_255", (0.212, 0.863, 1.0), metallic=0.1, roughness=0.18, emission=(0.08, 0.72, 1.0))

    create_surface(green, green_hi, green_dark)
    create_track((asphalt, white, black))
    torus("Race_Outer_Guardrail_N", 30.72, 0.22, yellow)
    torus("Race_Outer_Guardrail_S", 26.20, 0.22, yellow)
    torus("Race_Energy_Ring", 31.15, 0.10, cyan)
    create_boost_lights(cyan)
    create_polar_beacon(cyan, black)

    bpy.ops.wm.save_as_mainfile(filepath=str(OUTPUT / "planeta_carreras_detallado.blend"))
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.wm.obj_export(
        filepath=str(OUTPUT / "planeta_carreras_detallado.obj"),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        forward_axis="NEGATIVE_Z",
        up_axis="Y",
    )
    print("Created", OUTPUT / "planeta_carreras_detallado.obj")


if __name__ == "__main__":
    main()
