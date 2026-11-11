"""Create the five modular Blender environments for Lava Castle Part 2."""

from pathlib import Path
import math

import bpy


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "levels" / "lava_castle_interior"


def blender_location(location):
    """Convert the engine's Y-up coordinates to Blender's Z-up coordinates."""
    x, y, z = location
    return (x, -z, y)


def blender_scale(scale):
    x, y, z = scale
    return (x, z, y)


def blender_rotation(rotation):
    x, y, z = rotation
    return (x, -z, y)


def material(name, color, metallic=0.0, roughness=0.7, emission=None):
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


def move_to(obj, collection, name, mat, smooth=False):
    obj.name = name
    for source in list(obj.users_collection):
        source.objects.unlink(obj)
    collection.objects.link(obj)
    obj.data.materials.append(mat)
    if smooth:
        for polygon in obj.data.polygons:
            polygon.use_smooth = True
    return obj


def cube(collection, name, location, scale, mat, bevel=0.0, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cube_add(size=2, location=blender_location(location), rotation=blender_rotation(rotation))
    obj = move_to(bpy.context.object, collection, name, mat)
    obj.scale = blender_scale(scale)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        modifier = obj.modifiers.new("Volcanic bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 2
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    return obj


def cylinder(collection, name, location, radius, depth, mat, vertices=12, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=blender_location(location), rotation=blender_rotation(rotation))
    return move_to(bpy.context.object, collection, name, mat, True)


def torus(collection, name, location, major, minor, mat, rotation=(math.pi / 2, 0, 0)):
    bpy.ops.mesh.primitive_torus_add(major_radius=major, minor_radius=minor, major_segments=24, minor_segments=6, location=blender_location(location), rotation=blender_rotation(rotation))
    return move_to(bpy.context.object, collection, name, mat, True)


def cone(collection, name, location, radius, depth, mat, vertices=8, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cone_add(vertices=vertices, radius1=radius, radius2=0.05, depth=depth, location=blender_location(location), rotation=blender_rotation(rotation))
    return move_to(bpy.context.object, collection, name, mat)


def gothic_shell(collection, prefix, center_z, half_length, half_width, height, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, metal, rune = mats
    cube(collection, f"{prefix}_Floor", (0, 3, center_z), (half_width, 3, half_length), rock, 0.5)
    cube(collection, f"{prefix}_Ceiling", (0, height, center_z), (half_width, 3, half_length), brick_dark, 0.5)
    cube(collection, f"{prefix}_Wall_L", (-half_width, height / 2, center_z), (3, height / 2, half_length), rock_dark, 0.6)
    cube(collection, f"{prefix}_Wall_R", (half_width, height / 2, center_z), (3, height / 2, half_length), rock_dark, 0.6)
    for index, z in enumerate([center_z - half_length * .8, center_z - half_length * .4, center_z, center_z + half_length * .4, center_z + half_length * .8]):
        for side in (-1, 1):
            x = side * (half_width - 5)
            cube(collection, f"{prefix}_Torch_{side}_{index}", (side * (half_width - 8), height * .52, z), (0.7, 3.2, 0.7), lava_hot, 0.25)
        torus(collection, f"{prefix}_Arch_{index}", (0, height * .55, z), half_width - 7, 1.5, brick)
    # Hanging chains give the ceiling depth without expensive simulation.
    for chain in range(6):
        x = -25 + chain * 10
        for link in range(6):
            torus(collection, f"{prefix}_Chain_{chain}_{link}", (x, height - 7 - link * 3.2, center_z + (chain % 2) * 12 - 6), 1.0, .28, metal, (math.pi / 2, 0, (link % 2) * math.pi / 2))


def build_start(collection, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, metal, rune = mats
    gothic_shell(collection, "StartHall", 0, 55, 48, 78, mats)
    cylinder(collection, "StartHall_Landing", (0, 7, 24), 18, 3, brick, 24)
    torus(collection, "StartHall_Landing_Rune", (0, 9, 24), 13, 1.0, lava_hot, (0, 0, 0))
    for side in (-1, 1):
        cube(collection, f"StartHall_Banner_{side}", (side * 26, 52, 12), (5, 13, .35), rune, .2)
        cone(collection, f"StartHall_Banner_Point_{side}", (side * 26, 37, 12), 5, 8, rune, 5)
        cylinder(collection, f"StartHall_DragonPedestal_{side}", (side * 34, 14, -24), 7, 12, brick, 12)
        cone(collection, f"StartHall_Dragon_{side}", (side * 34, 28, -24), 7, 18, rock, 10, (0, 0, side * .25))


def build_furnace(collection, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, metal, rune = mats
    gothic_shell(collection, "MoltenFurnace", -105, 58, 48, 78, mats)
    for index, x in enumerate((-25, 0, 25)):
        z = -125 + index * 20
        torus(collection, f"Furnace_Ring_A_{index}", (x, 22, z), 11, 1.2, brick, (0, 0, 0))
        torus(collection, f"Furnace_Ring_B_{index}", (x, 41, z), 11, 1.2, brick, (0, 0, 0))
        cylinder(collection, f"Furnace_Pipe_{index}", (x, 58, z + 12), 2.2, 28, metal, 10, (math.pi / 2, 0, 0))
    for index, z in enumerate((-154, -130, -106, -82, -58)):
        cube(collection, f"Furnace_LavaChannel_{index}", (0, 7, z), (43, 1, 4.5), lava, .3)
        for side in (-1, 1):
            cylinder(collection, f"Furnace_GearHub_{index}_{side}", (side * 37, 37, z), 5, 3, metal, 12, (math.pi / 2, 0, 0))
            torus(collection, f"Furnace_Gear_{index}_{side}", (side * 37, 37, z), 9, 1.4, brick)


def build_sanctums(collection, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, metal, rune = mats
    gothic_shell(collection, "FragmentSanctums", -215, 56, 50, 82, mats)
    data = [(-12, 30, -215, (0.08, .42, 1.0), "Blue"), (0, 37, -215, (1.0, .22, .02), "Orange"), (12, 33.5, -215, (.58, .08, 1.0), "Purple")]
    for x, y, z, color, label in data:
        glow = material(f"race_rgb_{int(color[0]*255)}_{int(color[1]*255)}_{int(color[2]*255)}", color, .1, .15, color)
        cylinder(collection, f"Sanctum_{label}_Pedestal", (x, y - 6, z), 7, 4, brick, 18)
        torus(collection, f"Sanctum_{label}_Rune", (x, y - 3.5, z), 5.2, .55, glow, (0, 0, 0))
        for side in (-1, 1):
            cube(collection, f"Sanctum_{label}_Arch_{side}", (x + side * 8, y + 8, z), (1.6, 13, 1.6), brick, .35)
            cone(collection, f"Sanctum_{label}_Spire_{side}", (x + side * 8, y + 23, z), 2.5, 8, glow, 8)
    for direction in (-1, 1):
        cube(collection, f"Sanctum_SideBridge_{direction}", (direction * 27, 22, -215), (16, 2, 10), brick, .5)


def build_corridor(collection, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, metal, rune = mats
    gothic_shell(collection, "SealCorridor", -333, 66, 39, 72, mats)
    cube(collection, "SealCorridor_Bridge", (0, 8, -333), (34, 1.2, 64), brick, .4)
    for index, z in enumerate((-288, -316, -344, -372)):
        torus(collection, f"SealCorridor_MechanicalRing_{index}", (0, 36, z), 29, 2.2, metal)
        for spoke in range(8):
            angle = spoke * math.tau / 8
            cube(collection, f"SealCorridor_RingRune_{index}_{spoke}", (math.cos(angle)*29, 36 + math.sin(angle)*29, z - 1), (.7, 2.5, .45), lava_hot, .2, (0, 0, angle))
    for side in (-1, 1):
        cube(collection, f"SealCorridor_LavaRiver_{side}", (side * 34, 5.5, -333), (5, 1.2, 64), lava, .3)
    # Monumental seal frame remains static; the gameplay door is animated separately.
    for side in (-1, 1):
        cube(collection, f"SealCorridor_GateFrame_{side}", (side * 37, 38, -382), (4, 38, 6), brick_dark, .8)
    torus(collection, "SealCorridor_GateCrown", (0, 43, -382), 35, 3, brick)


def build_chamber(collection, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, metal, rune = mats
    gothic_shell(collection, "DragonBallChamber", -452, 58, 52, 88, mats)
    cylinder(collection, "DragonBallChamber_Altar", (0, 9, -470), 24, 5, brick, 28)
    torus(collection, "DragonBallChamber_AltarRune", (0, 12, -470), 18, 1.0, lava_hot, (0, 0, 0))
    torus(collection, "DragonBallChamber_InnerRune", (0, 12.2, -470), 10, .55, lava, (0, 0, 0))
    for side in (-1, 1):
        cone(collection, f"DragonBallChamber_DragonBody_{side}", (side * 43, 48, -474), 8, 17, rock, 10, (0, 0, side * .28))
        cone(collection, f"DragonBallChamber_DragonHorn_{side}", (side * 39, 56, -474), 2.2, 13, metal, 8, (0, side * .6, side * .7))
        for fall in range(3):
            cube(collection, f"DragonBallChamber_Lavafall_{side}_{fall}", (side * (44 - fall * 5), 43, -450 + fall * 17), (1.3, 35, 2.4), lava, .25)
    for ring in range(3):
        torus(collection, f"DragonBallChamber_EnergyRing_{ring}", (0, 36, -470), 9 + ring * 5, .35, lava_hot, (ring * .45, ring * .2, 0))


def build_lava_sentinel(collection, mats):
    rock, rock_dark, brick, brick_dark, lava, lava_hot, metal, rune = mats
    # Compact demonic drone designed to remain readable at gameplay distance.
    cylinder(collection, "LavaSentinel_CoreCage", (0, 0, 0), 3.5, 5.5, metal, 12, (math.pi / 2, 0, 0))
    torus(collection, "LavaSentinel_CoreRing", (0, 0, -3.0), 2.8, .55, lava_hot)
    cylinder(collection, "LavaSentinel_EnergyCore", (0, 0, -3.5), 2.0, 2.0, lava, 14, (math.pi / 2, 0, 0))
    cube(collection, "LavaSentinel_ArmorTop", (0, 2.4, 0), (3.8, 1.2, 3.5), rock_dark, .7)
    cube(collection, "LavaSentinel_ArmorBottom", (0, -2.4, .4), (3.3, 1.0, 3.0), rock, .6)
    for side in (-1, 1):
        cube(collection, f"LavaSentinel_WingRoot_{side}", (side * 4.2, 0, .5), (2.2, .65, 1.5), metal, .35, (0, 0, side * .2))
        cone(collection, f"LavaSentinel_WingBlade_{side}", (side * 7.0, .2, 1.0), 2.2, 7.5, rock_dark, 7, (0, side * math.pi / 2, side * .25))
        cone(collection, f"LavaSentinel_Horn_{side}", (side * 2.7, 4.2, .4), 1.0, 4.8, metal, 7, (0, 0, side * .55))
        cone(collection, f"LavaSentinel_Claw_{side}", (side * 4.5, -3.0, -1.0), .8, 4.0, lava_hot, 6, (0, side * .4, side * .25))
        cylinder(collection, f"LavaSentinel_Thruster_{side}", (side * 2.3, 0, 3.2), 1.1, 2.6, metal, 10, (math.pi / 2, 0, 0))
        cone(collection, f"LavaSentinel_ThrusterFlame_{side}", (side * 2.3, 0, 5.2), 1.0, 3.5, lava_hot, 8, (math.pi / 2, 0, 0))
    for eye in (-1, 1):
        cylinder(collection, f"LavaSentinel_Eye_{eye}", (eye * 1.35, 1.1, -3.75), .5, .45, lava_hot, 8, (math.pi / 2, 0, 0))


def export_collection(collection, path):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in collection.objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = next(iter(collection.objects))
    bpy.ops.wm.obj_export(filepath=str(path), export_selected_objects=True, export_materials=True, export_triangulated_mesh=True, forward_axis="NEGATIVE_Z", up_axis="Y")


def optimize_collection(collection, ratio=0.32):
    for obj in collection.objects:
        if obj.type != "MESH" or len(obj.data.polygons) < 24:
            continue
        modifier = obj.modifiers.new("Runtime optimization", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = ratio
        modifier.use_collapse_triangulate = True
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.modifier_apply(modifier=modifier.name)
        obj.select_set(False)


def main():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    mats = (
        material("Castle_Rock", (.15, .11, .12), .1, .88),
        material("Castle_Rock_Dark", (.065, .04, .055), .15, .9),
        material("Castle_Bricks", (.23, .18, .2), .18, .72),
        material("Castle_Bricks_Dark", (.085, .052, .072), .25, .78),
        material("Castle_Lava", (1.0, .06, .005), .02, .2, (1.0, .03, 0)),
        material("Castle_Lava_Hot", (1.0, .42, .015), .02, .15, (1.0, .22, .005)),
        material("Castle_Pole", (.055, .045, .07), .88, .24),
        material("race_rgb_125_12_18", (.49, .047, .07), .1, .62),
    )
    builders = [build_start, build_furnace, build_sanctums, build_corridor, build_chamber]
    names = ["start_hall", "molten_furnace", "fragment_sanctums", "seal_gate_corridor", "dragon_ball_chamber"]
    collections = []
    for builder, name in zip(builders, names):
        collection = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(collection)
        builder(collection, mats)
        optimize_collection(collection)
        collections.append(collection)
    enemy_collection = bpy.data.collections.new("lava_sentinel")
    bpy.context.scene.collection.children.link(enemy_collection)
    build_lava_sentinel(enemy_collection, mats)
    optimize_collection(enemy_collection, 0.55)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUTPUT / "lava_castle_interior.blend"))
    for collection, name in zip(collections, names):
        export_collection(collection, OUTPUT / f"{name}.obj")
    for collection, name in zip(collections, names):
        optimize_collection(collection, 0.38)
        export_collection(collection, OUTPUT / f"{name}_lod.obj")
    export_collection(enemy_collection, OUTPUT / "lava_sentinel.obj")
    print("Created Lava Castle Interior Blender assets in", OUTPUT)


if __name__ == "__main__":
    main()
