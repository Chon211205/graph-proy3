"""Create the optimized mechanical enemies used by Aquatic Ruins.

Run with Blender 4.x:
    blender --background --python scripts/create_aquatic_enemies_blender.py
"""

from __future__ import annotations

import math
from pathlib import Path

import bpy
from mathutils import Matrix, Vector


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "enemies"


def material(name, color, metallic=0.0, roughness=0.42, emission=None):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1.0)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if emission:
        shader.inputs["Emission Color"].default_value = (*emission, 1.0)
        shader.inputs["Emission Strength"].default_value = 6.0
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


def ico(name, location, scale, mat, subdivisions=2):
    bpy.ops.mesh.primitive_ico_sphere_add(
        subdivisions=subdivisions, radius=1.0, location=location
    )
    obj = finish(bpy.context.object, name, mat)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return obj


def cylinder(name, location, radius, depth, mat, rotation=(0.0, 0.0, 0.0), vertices=12):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation
    )
    return finish(bpy.context.object, name, mat)


def cone_between(name, start, end, radius, mat, vertices=8):
    start, end = Vector(start), Vector(end)
    direction = end - start
    midpoint = (start + end) * 0.5
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius,
        radius2=0.025,
        depth=direction.length,
        location=midpoint,
    )
    obj = finish(bpy.context.object, name, mat, smooth=False)
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0.0, 0.0, 1.0)).rotation_difference(direction)
    return obj


def cube(name, location, scale, mat, rotation=(0.0, 0.0, 0.0), bevel=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rotation)
    obj = finish(bpy.context.object, name, mat, smooth=False)
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        modifier = obj.modifiers.new("Edge bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    return obj


def create_puffer(mats):
    dark, metal, orange, yellow, red = mats
    prefix = "ENEMY_PUFFER_MINE"
    ico(prefix + "_Armored_Core", (0, 0, 0), (1.12, 1.12, 1.06), dark, 3)
    # Overlapping armor bands give the mine a manufactured, pressure-resistant shell.
    for index, z in enumerate((-0.62, 0.0, 0.62)):
        bpy.ops.mesh.primitive_torus_add(
            major_radius=0.91 if z == 0 else 0.72,
            minor_radius=0.11,
            major_segments=16,
            minor_segments=6,
            location=(0, 0, z),
        )
        finish(bpy.context.object, prefix + f"_Armor_Band_{index}", metal)
    directions = [
        (1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0),
        (0, 0, 1), (0, 0, -1), (0.70, 0.70, 0.25),
        (-0.70, 0.70, -0.25), (0.70, -0.70, -0.25),
        (-0.70, -0.70, 0.25), (0.55, 0.20, 0.82),
        (-0.55, -0.20, 0.82), (0.20, -0.55, -0.82),
        (-0.20, 0.55, -0.82),
    ]
    for index, direction in enumerate(directions):
        direction = Vector(direction).normalized()
        cone_between(
            prefix + f"_Warning_Spike_{index:02}",
            direction * 0.82,
            direction * 1.72,
            0.20,
            orange if index % 2 else yellow,
        )
    for index, angle in enumerate((0.0, math.pi * 0.5, math.pi, math.pi * 1.5)):
        location = (math.cos(angle) * 1.04, math.sin(angle) * 1.04, 0.18)
        uv(prefix + f"_Red_Sensor_{index}", location, (0.18, 0.18, 0.18), red, 12, 6)


def create_shark(mats):
    blue, light, dark, metal, red, cyan, tooth = mats
    prefix = "ENEMY_SHARK_DRONE"
    uv(prefix + "_Pressure_Hull", (0.0, 0, 0.15), (2.20, 0.82, 0.78), blue, 20, 10)
    uv(prefix + "_Armored_Back", (-0.15, 0, 0.54), (1.48, 0.70, 0.34), light, 16, 8)
    uv(prefix + "_Snout", (1.72, 0, 0.03), (0.76, 0.70, 0.52), light, 16, 8)
    cube(prefix + "_Lower_Jaw", (1.48, 0, -0.43), (0.78, 0.58, 0.16), dark, bevel=0.10)
    # Two rows of clearly readable teeth around the glowing weapon cavity.
    for side in (-1.0, 1.0):
        for index in range(5):
            x = 1.12 + index * 0.24
            start = (x, side * 0.48, -0.22)
            end = (x - 0.02, side * 0.24, -0.28)
            cone_between(prefix + f"_Tooth_{side}_{index}", start, end, 0.09, tooth, 6)
    uv(prefix + "_Mouth_Reactor", (1.72, 0, -0.24), (0.32, 0.32, 0.32), red, 14, 7)
    for side in (-1.0, 1.0):
        uv(prefix + f"_Eye_{side}", (1.64, side * 0.59, 0.34), (0.18, 0.10, 0.15), red, 12, 6)
        # Swept pectoral fins.
        fin_y = side * 1.35
        cone_between(
            prefix + f"_Pectoral_Fin_{side}",
            (0.20, side * 0.58, 0.02),
            (-0.45, fin_y, -0.22),
            0.36,
            metal,
            4,
        )
        cylinder(
            prefix + f"_Turbine_{side}",
            (-1.40, side * 0.66, 0.05),
            0.33,
            0.60,
            dark,
            rotation=(0, math.pi / 2, 0),
            vertices=12,
        )
        uv(prefix + f"_Engine_Glow_{side}", (-1.72, side * 0.66, 0.05), (0.10, 0.27, 0.27), cyan, 12, 6)
        for missile in (-0.18, 0.18):
            cylinder(
                prefix + f"_Missile_{side}_{missile}",
                (-0.20, side * 0.88, 0.58 + missile),
                0.09,
                0.62,
                light,
                rotation=(0, math.pi / 2, 0),
                vertices=8,
            )
    cone_between(prefix + "_Dorsal_Fin", (-0.35, 0, 0.58), (-0.72, 0, 1.52), 0.42, metal, 4)
    cylinder(prefix + "_Tail_Joint", (-2.10, 0, 0.08), 0.32, 0.76, dark, rotation=(0, math.pi / 2, 0))
    cone_between(prefix + "_Tail_Upper", (-2.35, 0, 0.08), (-2.72, 0, 1.15), 0.38, blue, 4)
    cone_between(prefix + "_Tail_Lower", (-2.35, 0, 0.08), (-2.70, 0, -0.93), 0.34, blue, 4)

    # Enemies travel toward decreasing level X; turn the complete assembly so
    # the snout, weapons and eyes face the player instead of pointing away.
    turn = Matrix.Rotation(math.pi, 4, "Z")
    for obj in bpy.context.scene.objects:
        if obj.name.startswith(prefix):
            obj.matrix_world = turn @ obj.matrix_world


def create_jelly(mats):
    purple, pink, dark, metal, cyan, yellow = mats
    prefix = "ENEMY_JELLY_BOT"
    uv(prefix + "_Dome", (0, 0, 0.45), (1.28, 1.28, 0.78), purple, 20, 10)
    # Hide the lower half behind a mechanical collar and bright energy core.
    bpy.ops.mesh.primitive_torus_add(
        major_radius=1.02, minor_radius=0.18, major_segments=20, minor_segments=6,
        location=(0, 0, 0.10)
    )
    finish(bpy.context.object, prefix + "_Mechanical_Collar", metal)
    uv(prefix + "_Energy_Core", (0, 0, 0.16), (0.48, 0.48, 0.48), cyan, 14, 7)
    for index in range(6):
        angle = index * math.tau / 6.0
        location = (math.cos(angle) * 0.77, math.sin(angle) * 0.77, 0.65)
        uv(prefix + f"_Pink_Node_{index}", location, (0.17, 0.17, 0.13), pink, 10, 5)
    for index in range(8):
        angle = index * math.tau / 8.0
        radius = 0.76 if index % 2 else 0.93
        x, y = math.cos(angle) * radius, math.sin(angle) * radius
        points = [
            (x, y, 0.02),
            (x * 1.10, y * 1.10, -0.52),
            (x * 0.78 + math.sin(angle) * 0.24, y * 0.78 - math.cos(angle) * 0.24, -1.10),
            (x * 0.95, y * 0.95, -1.72 - (index % 3) * 0.16),
        ]
        curve = bpy.data.curves.new(prefix + f"_Tentacle_Curve_{index}", "CURVE")
        curve.dimensions = "3D"
        curve.resolution_u = 1
        curve.bevel_depth = 0.10 if index % 2 else 0.13
        curve.bevel_resolution = 1
        spline = curve.splines.new("BEZIER")
        spline.bezier_points.add(len(points) - 1)
        for point, co in zip(spline.bezier_points, points):
            point.co = co
            point.handle_left_type = "AUTO"
            point.handle_right_type = "AUTO"
        obj = bpy.data.objects.new(prefix + f"_Tentacle_{index}", curve)
        bpy.context.collection.objects.link(obj)
        obj.data.materials.append(dark if index % 2 else purple)
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.convert(target="MESH")
        obj.select_set(False)
        uv(prefix + f"_Tentacle_Light_{index}", points[-1], (0.13, 0.13, 0.18), yellow, 10, 5)


def main():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    OUTPUT.mkdir(parents=True, exist_ok=True)

    dark = material("race_rgb_24_31_43", (0.094, 0.122, 0.169), 0.70, 0.25)
    metal = material("race_rgb_105_123_139", (0.412, 0.482, 0.545), 0.82, 0.24)
    light = material("race_rgb_205_222_229", (0.804, 0.871, 0.898), 0.68, 0.22)
    blue = material("race_rgb_31_105_151", (0.122, 0.412, 0.592), 0.62, 0.28)
    purple = material("race_rgb_105_52_151", (0.412, 0.204, 0.592), 0.48, 0.25)
    pink = material("race_rgb_235_62_166", (0.922, 0.243, 0.651), 0.20, 0.18, (1.0, 0.05, 0.45))
    orange = material("race_rgb_240_91_30", (0.941, 0.357, 0.118), 0.45, 0.25)
    yellow = material("race_rgb_255_204_48", (1.0, 0.8, 0.188), 0.32, 0.20, (1.0, 0.55, 0.03))
    red = material("race_rgb_255_35_48", (1.0, 0.137, 0.188), 0.30, 0.16, (1.0, 0.01, 0.02))
    cyan = material("race_rgb_35_225_255", (0.137, 0.882, 1.0), 0.20, 0.14, (0.02, 0.75, 1.0))
    tooth = material("race_rgb_242_247_240", (0.949, 0.969, 0.941), 0.10, 0.27)

    create_puffer((dark, metal, orange, yellow, red))
    create_shark((blue, light, dark, metal, red, cyan, tooth))
    create_jelly((purple, pink, dark, metal, cyan, yellow))

    # Use continuous vertex normals on every generated mesh. This also covers
    # low-sided fins, teeth, armor plates and spikes created with flat shading.
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            for polygon in obj.data.polygons:
                polygon.use_smooth = True

    blend_path = OUTPUT / "acuaticenemies_detallados.blend"
    obj_path = OUTPUT / "acuaticenemies.obj"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.wm.obj_export(
        filepath=str(obj_path),
        export_selected_objects=True,
        export_materials=True,
        export_triangulated_mesh=True,
        # The game uses X as forward, Y as lateral and Z as vertical. Keep
        # Blender's native Z-up basis so enemies do not arrive rolled 90°.
        forward_axis="NEGATIVE_Y",
        up_axis="Z",
    )
    print("Created", obj_path)


if __name__ == "__main__":
    main()
