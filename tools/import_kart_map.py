"""Convert the default kart_racer tile map to native OBJ assets.

Source checkout: target/kart_racer_source (see assets/levels/kart_racer/SOURCE.md).
Only map geometry is imported; no gameplay, menus or audio are copied.
"""
import re
import subprocess
from math import sin, cos, pi
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'target/kart_racer_source/src/main.rs'
OUT = ROOT / 'assets/levels/kart_racer'
SCALE = 6.0


class Mesh:
    def __init__(self):
        self.vertices, self.faces = [], []

    def part(self, group, rgb, points, polygons):
        offset = len(self.vertices)+1
        self.vertices.extend(points)
        material = 'race_rgb_'+'_'.join(map(str, rgb))
        for poly in polygons:
            for i in range(1,len(poly)-1):
                self.faces.append((group, material, [offset+poly[j] for j in (0,i,i+1)]))

    def box(self, group, rgb, x, y, z, sx, sy, sz):
        points = [(x+a*sx/2,y+b*sy/2,z+c*sz/2) for a,b,c in
                  [(-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),(-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)]]
        self.part(group,rgb,points,[(0,3,2,1),(4,5,6,7),(0,1,5,4),(3,7,6,2),(0,4,7,3),(1,2,6,5)])

    def sphere(self, group, rgb, center, radius, stacks=12, slices=20, colors=None):
        points=[]
        for j in range(stacks+1):
            lat=pi*j/stacks-pi/2
            for i in range(slices):
                a=2*pi*i/slices
                points.append((center[0]+radius*cos(lat)*cos(a),center[1]+radius*sin(lat),center[2]+radius*cos(lat)*sin(a)))
        for j in range(stacks):
            for i in range(slices):
                color=colors(j,i) if colors else rgb
                a=j*slices+i; b=j*slices+(i+1)%slices
                self.part(group,color,[points[a],points[b],points[b+slices],points[a+slices]],[(0,1,2,3)])

    def save(self,path):
        lines=['# Native OBJ geometry; provenance in SOURCE.md',f'mtllib {path.stem}.mtl']
        lines += [f'v {x:.5f} {y:.5f} {z:.5f}' for x,y,z in self.vertices]
        previous=None
        for group,material,face in self.faces:
            if (group,material)!=previous:
                lines += [f'g {group}',f'usemtl {material}']; previous=(group,material)
            lines.append('f '+' '.join(map(str,face)))
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text('\n'.join(lines)+'\n',encoding='utf-8')
        materials=[]
        for material in sorted({f[1] for f in self.faces}):
            rgb=list(map(int,material.removeprefix('race_rgb_').split('_')))
            materials += [f'newmtl {material}','Kd '+' '.join(f'{c/255:.5f}' for c in rgb),'']
        path.with_suffix('.mtl').write_text('\n'.join(materials),encoding='utf-8')


def main():
    revision=subprocess.check_output(['git','-C',str(SOURCE.parents[1]),'rev-parse','HEAD'],text=True).strip()
    if revision!='0f1605af68529dfe85607d64c9c05a848792dd88':
        raise SystemExit('Use the source revision recorded in assets/levels/kart_racer/SOURCE.md')
    block=SOURCE.read_text(encoding='utf-8').split('const MAP_GARDEN:')[1].split('];')[0]
    rows=re.findall(r'"([#.A-Z]+)"',block)
    assert len(rows)==41 and all(len(row)==65 for row in rows)
    OUT.mkdir(parents=True,exist_ok=True)
    (OUT/'jardin_rust.tiles').write_text('\n'.join(rows)+'\n',encoding='utf-8')
    m=Mesh()
    for j,row in enumerate(rows):
        for i,tile in enumerate(row):
            x,z=(i+.5)*SCALE,(j+.5)*SCALE
            rgb={'P':(70,72,76),'L':(235,220,70),'W':(55,145,220),'Q':(155,195,210)}.get(tile,(65,125,70))
            if tile=='K': rgb=(242,243,239) if (i+j)%2==0 else (220,40,38)
            if tile=='M': rgb=(242,243,239) if (i+j)%2==0 else (45,45,48)
            if tile=='R': rgb=(255,230,55) if (i+j)%2==0 else (245,75,38)
            m.part('Tile_'+tile,rgb,[(x-3,0,z-3),(x+3,0,z-3),(x+3,0,z+3),(x-3,0,z+3)],[(0,1,2,3)])
            if tile in '#HS':
                rgb={'#':(118,124,137),'H':(30,96,42),'S':(163,167,175)}[tile]
                height={'#':8,'H':5,'S':2}[tile]
                m.box('Wall_'+tile,rgb,x,height/2,z,6,height,6)
            elif tile=='W':
                m.box('Fountain_water',(55,145,220),x,.35,z,6,.7,6)
            elif tile=='Q':
                m.box('Fountain_center',(155,195,210),x,3,z,1.2,6,1.2)
            elif tile=='T':
                m.box('Tree_trunk',(110,67,30),x,3,z,.8,6,.8)
                m.sphere('Tree_crown',(28,104,45),(x,7,z),3,8,12)
            elif tile=='F':
                m.box('Flower_bed',(174,69,138),x,.45,z,3,.9,3)
            elif tile=='B':
                m.box('Barrier',(222,55,42),x,1,z,4,2,1)
                m.box('Barrier_stripe',(245,244,229),x,1.3,z,4,.5,1.05)
            elif tile=='C':
                m.box('Cone',(246,137,25),x,.8,z,1,1.6,1)
            elif tile in 'OYN':
                m.box('Sign_post',(120,126,140),x,2.5,z,.25,5,.25)
                m.box('Track_sign',(255,211,36),x,4.8,z,2.6,1.7,.35)
            elif tile=='G':
                for step in range(3):
                    m.box('Grandstand',(130+step*25,146,168),x,1+step,z+step*1.1,5,2+step*2,1.5)
    m.save(OUT/'jardin_rust.obj')
    planet=Mesh()
    def planet_color(j,i):
        if j in (10,13): return (245,246,239) if i%2 else (30,34,42)
        if 10<j<13: return (245,208,42) if i%6==0 else (60,64,73)
        return (44,133,88) if (i+j)%4 else (58,160,107)
    planet.sphere('Racing_planet',(50,145,90),(0,0,0),28,24,48,planet_color)
    planet.save(ROOT/'assets/planets/planeta_carreras.obj')


if __name__=='__main__': main()
