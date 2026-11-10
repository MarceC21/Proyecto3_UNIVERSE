"""Nave de dos vigas (café / cobre / dorado). Solo el modelo, sin dependencias.

Uso: python3 crear_nave_cafe.py
Genera nave_cafe.obj y nave_cafe.mtl junto al script.

Mismo método que detallar_exterior.py y crear_estacion.py: todo son triángulos,
índices OBJ desde 1, un material por grupo de caras.

Convención de ejes (igual que nave.obj): +Z es el frente, -Z los motores,
+Y arriba y las alas se abren en X.

Partes:
  - 2 vigas paralelas (x = ±5.2), largo 56, con juntas, paneles y franja de
    cobre abajo. El frente de cada viga termina en una PUNTA (pirámide con
    la última parte dorada); atrás llevan una tobera con brillo ámbar.
  - Un collar más grueso en el centro de cada viga.
  - Conector cilíndrico entre las vigas (atraviesa ambas y sobresale).
  - 2 aletas doradas trapezoidales (una a cada lado) con relieves.
"""
from pathlib import Path
import math

CARPETA = Path(__file__).resolve().parent
vertices, caras = [], []


# ----------------------------------------------------------------------
# Álgebra y primitivas
# ----------------------------------------------------------------------
def resta(a, b): return [a[i] - b[i] for i in range(3)]
def punto3(a, b): return sum(a[i] * b[i] for i in range(3))


def newell(ps):
    nx = ny = nz = 0.0
    for i in range(len(ps)):
        a, b = ps[i], ps[(i + 1) % len(ps)]
        nx += (a[1] - b[1]) * (a[2] + b[2])
        ny += (a[2] - b[2]) * (a[0] + b[0])
        nz += (a[0] - b[0]) * (a[1] + b[1])
    return [nx, ny, nz]


def vert(p):
    vertices.append([float(c) for c in p])
    return len(vertices)


def quad(m, pts, ref=None, hacia=None):
    """Polígono convexo -> triángulos. Quita puntos repetidos (un cuadrado que
    se cierra en una punta queda como triángulo) y orienta la normal hacia
    `hacia`, o alejándola del punto interior `ref`."""
    lim = []
    for p in pts:
        if not lim or math.dist(p, lim[-1]) > 1e-6:
            lim.append(p)
    if len(lim) > 1 and math.dist(lim[0], lim[-1]) <= 1e-6:
        lim.pop()
    if len(lim) < 3:
        return
    d = hacia
    if d is None and ref is not None:
        c = [sum(p[k] for p in lim) / len(lim) for k in range(3)]
        d = resta(c, ref)
    if d is not None and punto3(newell(lim), d) < 0:
        lim.reverse()
    ids = [vert(p) for p in lim]
    for i in range(1, len(ids) - 1):
        caras.append((m, [ids[0], ids[i], ids[i + 1]]))


def caja(m, x0, x1, y0, y1, z0, z1):
    ref = ((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2)
    a, b, c, d = (x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0)
    e, f, g, h = (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)
    for ps in [(a, d, c, b), (e, f, g, h), (a, e, h, d), (b, c, g, f), (d, h, g, c), (a, b, f, e)]:
        quad(m, ps, ref=ref)


def bilineal(ps, u, v):
    return [((1-u)*(1-v)*ps[0][k] + u*(1-v)*ps[1][k]
             + u*v*ps[2][k] + (1-u)*v*ps[3][k]) for k in range(3)]


def parche(m, ps, u0, u1, v0, v1, off):
    """Rectángulo pintado sobre una cara, levantado `off` para evitar parpadeo."""
    pts = [bilineal(ps, u, v) for u, v in [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]]
    pts = [[p[k] + off[k] for k in range(3)] for p in pts]
    quad(m, pts, hacia=off)


def cilindro_x(m, x0, x1, r, cy, cz, segs=16):
    """Cilindro tumbado a lo largo de X, con tapas."""
    p = lambda x, k: (x, cy + r * math.cos(k * math.tau / segs), cz + r * math.sin(k * math.tau / segs))
    ref = ((x0 + x1) / 2, cy, cz)
    for k in range(segs):
        quad(m, [p(x0, k), p(x1, k), p(x1, k + 1), p(x0, k + 1)], ref=ref)
    quad(m, [p(x0, k) for k in range(segs)], ref=ref)
    quad(m, [p(x1, k) for k in range(segs)], ref=ref)


def prisma_xz(m, pts, y0, y1):
    """Prisma vertical a partir de un polígono convexo en el plano XZ."""
    cx = sum(p[0] for p in pts) / len(pts)
    cz = sum(p[1] for p in pts) / len(pts)
    ref = (cx, (y0 + y1) / 2, cz)
    quad(m[0], [(x, y1, z) for x, z in pts], ref=ref)          # cara de arriba
    quad(m[0], [(x, y0, z) for x, z in pts], ref=ref)          # cara de abajo
    for i in range(len(pts)):
        (xa, za), (xb, zb) = pts[i], pts[(i + 1) % len(pts)]
        quad(m[1], [(xa, y0, za), (xb, y0, zb), (xb, y1, zb), (xa, y1, za)], ref=ref)


def punta(x0, x1, y0, y1, zb, zt, tramos):
    """Punta del frente: secciones rectangulares que se encogen hasta un punto.
    tramos = [(t_final, mat_arriba, mat_lados, mat_abajo)], t en 0..1."""
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2

    def sec(t):
        k = 1 - t
        return (cx + (x0 - cx) * k, cx + (x1 - cx) * k,
                cy + (y0 - cy) * k, cy + (y1 - cy) * k, zb + (zt - zb) * t)

    t_ant = 0.0
    ref = (cx, cy, zb)
    for t, arriba, lados, abajo in tramos:
        a, b = sec(t_ant), sec(t)
        A = [(a[0], a[2], a[4]), (a[1], a[2], a[4]), (a[1], a[3], a[4]), (a[0], a[3], a[4])]
        B = [(b[0], b[2], b[4]), (b[1], b[2], b[4]), (b[1], b[3], b[4]), (b[0], b[3], b[4])]
        quad(abajo, [A[0], A[1], B[1], B[0]], ref=ref)    # y mínima
        quad(lados, [A[1], A[2], B[2], B[1]], ref=ref)    # x máxima
        quad(arriba, [A[2], A[3], B[3], B[2]], ref=ref)   # y máxima
        quad(lados, [A[3], A[0], B[0], B[3]], ref=ref)    # x mínima
        t_ant = t


# ----------------------------------------------------------------------
# Materiales (mismos nombres de glow que nave.mtl: Fondo_de_toberas brilla)
# ----------------------------------------------------------------------
CAFE, CAFE_OSC, COBRE = 'Casco_cafe', 'Casco_cafe_oscuro', 'Cobre'
DORADO, DORADO_OSC, PLACA = 'Dorado', 'Dorado_oscuro', 'Placa_cafe'
OSCURO, TOBERA = 'Refuerzos_oscuros', 'Fondo_de_toberas'

# ----------------------------------------------------------------------
# Medidas
# ----------------------------------------------------------------------
VIGA_X = 5.2            # centro de cada viga
AN, AL = 1.8, 1.5       # semiancho y semialto de la viga
Z_ATRAS, Z_FRENTE, Z_PUNTA = -30.0, 26.0, 36.0
COLLAR = (-4.0, 8.0)    # tramo central más grueso
Z_CENTRO = 2.0          # centro del collar, del conector y de las aletas
BAHIAS = [(-30, -22), (-22, -13), (-13, -4), (8, 15), (15, 22), (22, 26)]
JUNTAS = [-22, -13, 15, 22]


def viga(cx):
    x0, x1 = cx - AN, cx + AN
    caja(CAFE, x0, x1, -AL, AL, Z_ATRAS, Z_FRENTE)
    # franja de cobre bajo la viga
    caja(COBRE, x0 - .05, x1 + .05, -AL - .12, -AL + .40, Z_ATRAS + .1, Z_FRENTE - .1)
    # juntas
    for z in JUNTAS:
        caja(CAFE_OSC, x0 - .07, x1 + .07, -AL - .07, AL + .07, z - .15, z + .15)
    # paneles: arriba y los dos costados de cada tramo
    for za, zb in BAHIAS:
        za, zb = za + .4, zb - .4
        arriba = [(x0, AL, za), (x1, AL, za), (x1, AL, zb), (x0, AL, zb)]
        parche(OSCURO, arriba, .08, .92, .04, .96, [0, .015, 0])
        parche(PLACA, arriba, .14, .86, .10, .90, [0, .04, 0])
        for x, s in [(x1, 1), (x0, -1)]:
            lado = [(x, -AL, za), (x, -AL, zb), (x, AL, zb), (x, AL, za)]
            parche(OSCURO, lado, .05, .95, .12, .88, [s * .015, 0, 0])
            parche(PLACA, lado, .10, .90, .20, .80, [s * .04, 0, 0])
    # collar central: más alto y ancho, con dos tiras de cobre arriba
    c0, c1 = COLLAR
    caja(CAFE, x0 - .25, x1 + .25, -AL - .25, AL + .25, c0, c1)
    caja(CAFE_OSC, x0 - .30, x1 + .30, -AL - .30, AL + .30, c0 - .1, c0 + .15)
    caja(CAFE_OSC, x0 - .30, x1 + .30, -AL - .30, AL + .30, c1 - .15, c1 + .1)
    top = [(x0 - .25, AL + .25, c0 + .4), (x1 + .25, AL + .25, c0 + .4),
           (x1 + .25, AL + .25, c1 - .4), (x0 - .25, AL + .25, c1 - .4)]
    parche(PLACA, top, .10, .90, .06, .94, [0, .03, 0])
    for z in (c0 + 2.2, c1 - 2.6):
        caja(COBRE, x0 - .1, x1 + .1, AL + .27, AL + .33, z, z + .5)
    # tobera trasera: escalón oscuro + brillo ámbar
    caja(CAFE_OSC, x0 + .3, x1 - .3, -AL + .3, AL - .3, Z_ATRAS - 1.2, Z_ATRAS)
    caja(TOBERA, x0 + .65, x1 - .65, -AL + .65, AL - .65, Z_ATRAS - 1.35, Z_ATRAS - 1.2)
    # PUNTA al frente: café y, al final, un extremo dorado
    punta(x0, x1, -AL, AL, Z_FRENTE, Z_PUNTA,
          [(0.80, CAFE, CAFE_OSC, COBRE), (1.0, DORADO, DORADO_OSC, DORADO_OSC)])


for cx in (-VIGA_X, VIGA_X):
    viga(cx)

# ----------------------------------------------------------------------
# Conector entre las vigas: cilindro con anillos de cobre y remates
# ----------------------------------------------------------------------
X_FIN = VIGA_X + AN + 1.4
cilindro_x(CAFE, -X_FIN, X_FIN, 1.5, 0.0, Z_CENTRO)
for x0, x1 in [(2.2, 2.7), (-2.7, -2.2), (7.5, 8.0), (-8.0, -7.5)]:
    cilindro_x(COBRE, x0, x1, 1.78, 0.0, Z_CENTRO)
cilindro_x(CAFE_OSC, X_FIN, X_FIN + .25, 1.1, 0.0, Z_CENTRO)
cilindro_x(CAFE_OSC, -X_FIN - .25, -X_FIN, 1.1, 0.0, Z_CENTRO)

# ----------------------------------------------------------------------
# Aletas doradas: trapecios planos sobre el collar, una por cada lado
# ----------------------------------------------------------------------
X_BASE, X_PUNTA = 1.4, 13.7
HW_BASE, HW_PUNTA = 1.45, 0.90
Y_ALETA0, Y_ALETA1 = 1.76, 2.26


def hw(x):
    return HW_BASE + (HW_PUNTA - HW_BASE) * (x - X_BASE) / (X_PUNTA - X_BASE)


for s in (1, -1):
    pts = [(s * X_BASE, Z_CENTRO - HW_BASE), (s * X_PUNTA, Z_CENTRO - HW_PUNTA),
           (s * X_PUNTA, Z_CENTRO + HW_PUNTA), (s * X_BASE, Z_CENTRO + HW_BASE)]
    prisma_xz((DORADO, DORADO_OSC), pts, Y_ALETA0, Y_ALETA1)
    # puntal de cobre que la une al conector
    caja(COBRE, min(s * 1.3, s * 3.2), max(s * 1.3, s * 3.2), -1.0, Y_ALETA0,
         Z_CENTRO - 1.0, Z_CENTRO + 1.0)
    # relieves: una barra larga y una corta (como en la referencia)
    for (xa, xb, zc) in [(3.0, 12.6, -.38), (3.6, 7.2, .62)]:
        x0, x1 = sorted((s * xa, s * xb))
        caja(DORADO_OSC, x0, x1, Y_ALETA1, Y_ALETA1 + .07,
             Z_CENTRO + zc - .17, Z_CENTRO + zc + .17)

# ----------------------------------------------------------------------
# Escritura
# ----------------------------------------------------------------------
COLORES = {
    CAFE: (.58, .40, .22), CAFE_OSC: (.36, .24, .13), COBRE: (.74, .40, .16),
    DORADO: (.92, .76, .32), DORADO_OSC: (.68, .52, .20), PLACA: (.49, .34, .19),
    OSCURO: (.12, .09, .07), TOBERA: (1.0, .66, .26),
}
salida = ['# Nave de dos vigas triangulada\nmtllib nave_cafe.mtl\no NaveCafe\n']
salida += ['v %.6f %.6f %.6f\n' % tuple(v) for v in vertices]
for m in COLORES:
    salida.append(f'usemtl {m}\n')
    for mat, ids in caras:
        if mat == m:
            salida.append(f'f {ids[0]} {ids[1]} {ids[2]}\n')
(CARPETA / 'nave_cafe.obj').write_text(''.join(salida))
(CARPETA / 'nave_cafe.mtl').write_text(''.join(
    f'newmtl {m}\nKd {r} {g} {b}\n\n' for m, (r, g, b) in COLORES.items()))
print(f'Nave café: {len(vertices)} vértices, {len(caras)} triángulos')
