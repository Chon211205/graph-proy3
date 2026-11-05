"""Create Vesper's tower-house planet from scratch with Blender 4.x."""
import math, random
from pathlib import Path
import bpy

ROOT=Path(__file__).resolve().parents[1]; OUT=ROOT/"assets"/"planets"; RNG=random.Random(7319)

def mat(name,color,rough=.6,metal=0,emit=0,alpha=1,noise=None):
    m=bpy.data.materials.new(name); m.diffuse_color=(*color,alpha); m.use_nodes=True
    n=m.node_tree.nodes; l=m.node_tree.links; s=n["Principled BSDF"]
    s.inputs["Base Color"].default_value=(*color,1); s.inputs["Roughness"].default_value=rough
    s.inputs["Metallic"].default_value=metal; s.inputs["Alpha"].default_value=alpha
    if emit: s.inputs["Emission Color"].default_value=(*color,1); s.inputs["Emission Strength"].default_value=emit
    if alpha<1:
        s.inputs["Transmission Weight"].default_value=.28
        if hasattr(m,"surface_render_method"): m.surface_render_method="DITHERED"
        else: m.blend_method="BLEND"
    if noise:
        t=n.new("ShaderNodeTexNoise"); t.inputs["Scale"].default_value=noise[0]; t.inputs["Detail"].default_value=noise[1]
        b=n.new("ShaderNodeBump"); b.inputs["Strength"].default_value=noise[2]; b.inputs["Distance"].default_value=.12
        l.new(t.outputs["Fac"],b.inputs["Height"]); l.new(b.outputs["Normal"],s.inputs["Normal"])
    return m

def finish(o,m,bevel=0,smooth=False):
    o.data.materials.append(m)
    for f in o.data.polygons: f.use_smooth=smooth
    if bevel: q=o.modifiers.new("Hand-worn edges","BEVEL"); q.width=bevel; q.segments=2
    return o

def cube(name,loc,size,m,bevel=.035,rot=(0,0,0)):
    bpy.ops.mesh.primitive_cube_add(size=1,location=loc,rotation=rot); o=finish(bpy.context.object,m,bevel); o.name=name; o.dimensions=size
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True); return o

def cyl(name,loc,r,depth,m,verts=20,rot=(0,0,0),bevel=.025):
    bpy.ops.mesh.primitive_cylinder_add(vertices=verts,radius=r,depth=depth,location=loc,rotation=rot)
    o=finish(bpy.context.object,m,bevel,True); o.name=name; return o

def cone(name,loc,r1,r2,depth,m,verts=24,rot=(0,0,0)):
    bpy.ops.mesh.primitive_cone_add(vertices=verts,radius1=r1,radius2=r2,depth=depth,location=loc,rotation=rot)
    o=finish(bpy.context.object,m,.025,True); o.name=name; return o

def asteroid(rock,moss,crystal):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=4,radius=3.15,location=(0,0,-2.25)); o=finish(bpy.context.object,rock,0,True); o.name="Tower_Planet_Asteroid"
    for v in o.data.vertices:
        p=v.co.normalized(); v.co*=1+.10*math.sin(p.x*8+p.z*3)*math.cos(p.y*11)+.045*math.sin((p.x+p.y)*19); v.co.z*=.82
    for i in range(18):
        a=math.tau*i/18+RNG.uniform(-.08,.08); r=RNG.uniform(1.75,2.75)
        cube(f"Moss_Terrace_{i:02}",(math.cos(a)*r,math.sin(a)*r,-.15+RNG.uniform(-.12,.12)),(RNG.uniform(.5,.95),RNG.uniform(.35,.65),RNG.uniform(.10,.20)),moss,.05,(0,RNG.uniform(-.18,.18),a))
    for i in range(11):
        a=math.tau*i/11+RNG.uniform(-.22,.22); r=RNG.uniform(.55,2.25)
        cone(f"Underside_Crystal_{i:02}",(math.cos(a)*r,math.sin(a)*r,-4.1-RNG.uniform(0,.5)),RNG.uniform(.12,.28),0,RNG.uniform(.8,1.8),crystal,6,(RNG.uniform(-.28,.28),RNG.uniform(-.28,.28),a))

def window(name,c,w,h,side,stone,glass,glow):
    x,y,z=c
    if not side:
        cube(name+"_Frame",c,(w+.18,.10,h+.14),stone,.025); cube(name+"_Glass",(x,y-.058,z),(w,.035,h),glass,.01)
        cyl(name+"_Arch",(x,y-.075,z+h/2),w/2,.045,glow,20,(math.pi/2,0,0),.008); cube(name+"_Mullion",(x,y-.09,z),(.035,.025,h),stone,.008)
    else:
        cube(name+"_Frame",c,(.10,w+.18,h+.14),stone,.025); cube(name+"_Glass",(x-.058,y,z),(.035,w,h),glass,.01)
        cyl(name+"_Arch",(x-.075,y,z+h/2),w/2,.045,glow,20,(0,math.pi/2,0),.008); cube(name+"_Mullion",(x-.09,y,z),(.025,.035,h),stone,.008)

def tower(stone,dark,trim,roof,wood,glass,glow,metal):
    stages=[((0,0,.30),(3.65,3.05,1.75)),((-.16,.08,1.88),(3.20,2.65,1.36)),((.12,-.04,3.16),(2.78,2.30,1.25)),((-.04,.05,4.39),(3.05,2.48,1.18))]
    for i,(loc,size) in enumerate(stages):
        cube(f"Tower_Stage_{i+1}",loc,size,stone if i!=2 else dark,.075); cube(f"Tower_Ledge_{i+1}",(loc[0],loc[1],loc[2]+size[2]/2),(size[0]+.30,size[1]+.30,.15),trim,.025)
        for sx in (-1,1):
            for sy in (-1,1):
                for row in range(max(2,int(size[2]/.28))):
                    cube(f"Quoin_{i}_{sx}_{sy}_{row}",(loc[0]+sx*(size[0]/2+.035),loc[1]+sy*(size[1]/2+.035),loc[2]-size[2]/2+.16+row*.28),(.22,.22,.17),trim,.018,(0,0,.05*(row%2)))
    for step in range(5): cube(f"Entry_Step_{step}",(.72,-1.82-step*.18,-.22+step*.10),(1.16,.42,.16),dark,.03)
    cube("Main_Door_Recess",(.72,-1.56,.34),(.94,.12,1.20),dark); cube("Main_Door",(.72,-1.64,.34),(.70,.10,1.03),wood)
    for z in (.02,.34,.66): cube(f"Door_Band_{z}",(.72,-1.71,z),(.75,.035,.045),metal,.008)
    rows=[(-.38,1.30,(-1.20,-.55)),(1.92,1.08,(-.93,-.31,.31,.93)),(3.20,.90,(-.72,0,.72)),(4.42,.92,(-.92,-.31,.31,.92))]
    for r,(z,h,xs) in enumerate(rows):
        for c,x in enumerate(xs): window(f"Front_Window_{r}_{c}",(x,-(stages[r][1][1]/2+.07),z),.34,h,0,trim,glass,glow)
    for r,(z,h,ys) in enumerate(((1.92,1.0,(-.62,.30)),(3.20,.84,(-.45,.42)),(4.42,.86,(-.62,.35)))):
        half=stages[r+1][1][0]/2+.07
        for s in (-1,1):
            for c,y in enumerate(ys): window(f"Side_Window_{r}_{s}_{c}",(s*half,y,z),.32,h,1,trim,glass,glow)
    for z,w in ((2.58,2.05),(5.02,2.20)):
        cube(f"Balcony_Floor_{z}",(0,-1.48,z),(w,.72,.14),dark)
        for x in (-(w/2-.12),0,w/2-.12): cyl(f"Balcony_Post_{z}_{x}",(x,-1.78,z+.34),.045,.66,metal,10)
        cube(f"Balcony_Rail_{z}",(0,-1.80,z+.66),(w,.06,.08),metal,.012)
    cone("Tower_Roof",(-.04,.05,5.86),2.18,.18,1.75,roof,4,(0,0,math.pi/4)); cube("Roof_Ridge",(-.04,.05,6.74),(3.25,.12,.12),metal,.02,(0,0,math.pi/4))
    for i,x in enumerate((-1.02,0,1.02)):
        cube(f"Roof_Dormer_{i}",(x,-1.28,5.64),(.58,.48,.64),stone); cube(f"Roof_Dormer_Glass_{i}",(x,-1.54,5.64),(.34,.035,.37),glow,.008)
        cone(f"Roof_Dormer_Cap_{i}",(x,-1.30,6.04),.47,.04,.46,roof,4,(0,0,math.pi/4))
    for i,(x,y,h) in enumerate(((-1.10,.52,1.18),(1.05,.44,.92))):
        cube(f"Chimney_{i}",(x,y,5.87+h/2),(.34,.34,h),dark); cube(f"Chimney_Cap_{i}",(x,y,5.87+h),(.46,.46,.16),trim)
    cyl("Observation_Turret",(1.46,.76,5.12),.58,1.55,stone,18)
    for a in range(0,360,90):
        q=math.radians(a); cube(f"Turret_Window_{a}",(1.46+math.cos(q)*.56,.76+math.sin(q)*.56,5.22),(.24,.05,.46),glow,.01,(0,0,q+math.pi/2))
    cone("Observation_Turret_Roof",(1.46,.76,6.18),.78,.05,.70,roof,18); cyl("Turret_Finial",(1.46,.76,6.72),.045,.50,metal,10)

def main():
    bpy.ops.object.select_all(action="SELECT"); bpy.ops.object.delete(use_global=False); OUT.mkdir(parents=True,exist_ok=True)
    rock=mat("tower_rock",(.22,.20,.24),.95,noise=(3.1,7,.55)); moss=mat("tower_moss",(.12,.26,.16),.92,noise=(8,4,.18))
    stone=mat("tower_stone",(.43,.45,.47),.83,noise=(7,5,.32)); dark=mat("tower_stone_dark",(.24,.25,.29),.88,noise=(5,6,.34)); trim=mat("tower_trim",(.63,.61,.57),.72,noise=(11,3,.15))
    roof=mat("tower_roof",(.12,.025,.22),.38,.12,noise=(14,2,.12)); wood=mat("tower_wood",(.25,.09,.025),.66,noise=(4,3,.24)); glass=mat("tower_glass",(.025,.12,.22),.12,.08,.4,.68)
    glow=mat("tower_window_glow",(.10,.55,1),.15,.05,5.5); metal=mat("tower_iron",(.055,.065,.085),.30,.72); crystal=mat("tower_crystal",(.18,.06,.42),.22,.18,2.8)
    asteroid(rock,moss,crystal); tower(stone,dark,trim,roof,wood,glass,glow,metal)
    col=bpy.data.collections.new("Vesper Tower Planet"); bpy.context.scene.collection.children.link(col)
    for o in list(bpy.context.scene.collection.objects): bpy.context.scene.collection.objects.unlink(o); col.objects.link(o)
    bpy.context.scene.world.color=(.004,.002,.012); bpy.ops.wm.save_as_mainfile(filepath=str(OUT/"caracola.blend"))
    bpy.context.view_layer.update()
    bpy.ops.wm.obj_export(filepath=str(OUT/"caracola.obj"),export_selected_objects=False,export_materials=True,export_triangulated_mesh=True,forward_axis="NEGATIVE_Z",up_axis="Y")
    print("Created new Vesper tower planet",OUT/"caracola.obj")
if __name__=="__main__": main()
