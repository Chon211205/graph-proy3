"""Rebuild the three original Buzz-inspired OBJ assets (standard library only).

Axes match ASTRA: +Z forward, +Y up. Materials also work in external OBJ viewers.
"""
from math import sin, cos, pi
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / 'assets' / 'vehicles'
PALETTE = {
    'buzz_white': (238, 242, 248), 'buzz_green': (153, 226, 38),
    'buzz_purple': (116, 61, 177), 'buzz_red': (235, 46, 60),
    'buzz_glass': (80, 177, 200), 'buzz_dark': (29, 36, 54),
    'buzz_blue': (45, 132, 240), 'buzz_gold': (245, 190, 42),
}


class Mesh:
    def __init__(self):
        self.vertices, self.faces = [], []

    def part(self, name, mat, vertices, faces):
        offset = len(self.vertices) + 1
        self.vertices.extend(vertices)
        for face in faces:
            for i in range(1, len(face) - 1):
                self.faces.append((name, mat, [offset + face[j] for j in (0, i, i + 1)]))

    def box(self, name, mat, center, size):
        x, y, z = center
        a, b, c = (v / 2 for v in size)
        self.part(name, mat, [(x+dx*a, y+dy*b, z+dz*c)
                  for dx, dy, dz in [(-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),
                                     (-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)]],
                  [(0,3,2,1),(4,5,6,7),(0,1,5,4),(3,7,6,2),(0,4,7,3),(1,2,6,5)])

    def ellipsoid(self, name, mat, center, size):
        vertices = []
        for row in range(13):
            lat = -pi/2 + pi*row/12
            for col in range(24):
                lon = 2*pi*col/24
                vertices.append((center[0]+size[0]*cos(lat)*cos(lon),
                                 center[1]+size[1]*sin(lat),
                                 center[2]+size[2]*cos(lat)*sin(lon)))
        faces = [(r*24+c, r*24+(c+1)%24, (r+1)*24+(c+1)%24, (r+1)*24+c)
                 for r in range(12) for c in range(24)]
        self.part(name, mat, vertices, faces)

    def wing(self, name, mat, side, y, inner, outer, front, rear):
        # Swept, tapered wing with a solid edge, rather than a single plane.
        outline = [(side*inner, y, front), (side*outer, y, front-1.6),
                   (side*outer, y, rear-0.5), (side*inner, y, rear)]
        vertices = [(x, h+dy, z) for dy in (-0.16, 0.16) for x,h,z in outline]
        self.part(name, mat, vertices,
                  [(0,1,2,3),(4,7,6,5),(0,4,5,1),(1,5,6,2),(2,6,7,3),(3,7,4,0)])

    def save(self, filename):
        lines = ['# Original Buzz Lightyear inspired vehicle', 'mtllib buzz.mtl']
        lines += [f'v {x:.5f} {y:.5f} {z:.5f}' for x,y,z in self.vertices]
        previous = None
        for name, mat, face in self.faces:
            if (name, mat) != previous:
                lines += [f'g {name}', f'usemtl {mat}']
                previous = name, mat
            lines.append('f ' + ' '.join(map(str, face)))
        (OUT / filename).write_text('\n'.join(lines)+'\n', encoding='utf-8')


def build(kind):
    m = Mesh()
    car, sub = kind == 'carro', kind == 'submarino'
    m.ellipsoid('Fuselaje', 'buzz_white', (0,0,0), (1.65,0.85,3.6))
    m.box('Quilla_morada', 'buzz_purple', (0,-0.62,-0.1), (2.7,0.35,4.4))
    m.ellipsoid('Proa_verde', 'buzz_green', (0,0.12,2.65), (1.15,0.50,1.1))
    m.ellipsoid('Aro_casco', 'buzz_purple', (0,0.75,0.6), (1.22,0.38,1.65))
    m.ellipsoid('Cabina_casco', 'buzz_glass', (0,1.05,0.65), (1.04,0.85,1.42))
    m.box('Franja_cabina', 'buzz_white', (0,1.87,0.5), (0.16,0.10,1.2))
    m.box('Pecho_verde', 'buzz_green', (0,0.78,2.15), (1.65,0.18,0.65))
    for x, mat in [(-0.48,'buzz_blue'), (0,'buzz_green'), (0.48,'buzz_red')]:
        m.box('Boton_pecho', mat, (x,0.90,2.15), (0.26,0.12,0.3))
    m.box('Insignia_ranger', 'buzz_gold', (0,0.86,1.65), (0.8,0.12,0.20))
    for side in (-1, 1):
        x = side*1.8
        m.ellipsoid('Motor_blanco', 'buzz_white', (x,0,-1.35), (0.70,0.68,1.8))
        m.ellipsoid('Hombro_verde', 'buzz_green', (x,0.2,-0.3), (0.73,0.56,0.85))
        m.ellipsoid('Tobera_morada', 'buzz_purple', (x,0,-2.7), (0.57,0.57,0.4))
        m.ellipsoid('Motor_luminoso', 'buzz_green', (x,0,-2.98), (0.37,0.37,0.10))
        m.box('Laser_rojo', 'buzz_red', (side*1.40,0.36,1.25), (0.22,0.22,1.3))
        if car:
            for z in (-2, 2):
                m.ellipsoid('Neumatico', 'buzz_dark', (side*2,-0.75,z), (0.58,0.85,0.85))
                m.ellipsoid('Llanta_morada', 'buzz_purple', (side*2.49,-0.75,z), (0.12,0.55,0.55))
                m.ellipsoid('Centro_verde', 'buzz_green', (side*2.60,-0.75,z), (0.07,0.25,0.25))
            m.box('Faro', 'buzz_gold', (side*1,0,3.25), (0.5,0.25,0.25))
            m.box('Aleron_soporte', 'buzz_purple', (side*1.2,0.7,-2.5), (0.18,1.1,0.3))
        else:
            reach = 3.6 if sub else 6.4
            m.wing('Ala_morada', 'buzz_purple', side,0,1.35,reach,0.6,-1.6)
            m.wing('Ala_blanca', 'buzz_white', side,0.19,1.5,reach-0.2,0.48,-1.4)
            # Red/white striped outer wings recall Buzz's deployable wings.
            for n in range(5):
                a = 2.15+n*(reach-2.15)/5
                b = a+(reach-2.15)/10
                m.wing('Franja_roja', 'buzz_red', side,0.37,a,b,0.10,-1.0)
            m.box('Punta_verde', 'buzz_green', (side*reach,0.35,-1), (0.25,0.95,1.6))
            if sub:
                m.ellipsoid('Flotador', 'buzz_green', (side*2.6,-0.45,-0.2), (0.55,0.55,2.5))
                m.box('Luz_submarina', 'buzz_blue', (side*1,0,3.15), (0.45,0.35,0.35))
    if car:
        m.box('Aleron', 'buzz_green', (0,1.25,-2.6), (4.5,0.2,0.7))
        m.box('Parachoques', 'buzz_purple', (0,-0.4,3.1), (2.8,0.35,0.3))
    if sub:
        m.box('Periscopio', 'buzz_purple', (0,2,-0.85), (0.22,1.35,0.22))
        m.box('Visor_periscopio', 'buzz_green', (0,2.65,-0.55), (0.55,0.3,0.85))
        m.box('Aleta_dorsal', 'buzz_purple', (0,1.0,-2.2), (0.18,1.5,1.4))
    m.save(f'buzz_{kind}.obj')


if __name__ == '__main__':
    OUT.mkdir(parents=True, exist_ok=True)
    material = []
    for name, color in PALETTE.items():
        material += [f'newmtl {name}', 'Kd '+' '.join(f'{c/255:.4f}' for c in color), '']
    (OUT/'buzz.mtl').write_text('\n'.join(material), encoding='utf-8')
    for kind in ('nave', 'carro', 'submarino'):
        build(kind)
