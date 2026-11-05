"""Estación espacial (solo el modelo), sin Blender ni dependencias.

Uso: python3 crear_estacion.py
Genera estacion.obj y estacion.mtl junto al script.

Mismo estilo que detallar_exterior.py:
 - todo se exporta como triángulos, índices OBJ desde 1;
 - mismos nombres de material que nave.mtl, así render.rs los reconoce
   (Luz_cian y *toberas brillan solas; Vidrio y Reflejo son más claros).

Ejes (igual que la nave): x, z = plano horizontal, y = arriba.
  - Anillo exterior: radio 16..20, altura -2..2 (48 facetas).
  - Núcleo (hub): prisma de 12 lados en el centro, con banda de ventanas,
    techo escalonado, 3 platos de antena, torre y mástiles. Un tallo abajo.
  - 4 brazos de módulos (ejes X y Z) que unen el núcleo con el anillo.
  - 4 brazos de paneles solares en las diagonales (2x2 paneles cada uno).
  - 4 balizas cuadradas en el techo del anillo y un propulsor en el costado.
"""
from pathlib import Path
import math

CARPETA = Path(__file__).resolve().parent
vertices, caras = [], []          # caras: (material, [a, b, c]) con índices desde 1


# ----------------------------------------------------------------------
# Álgebra mínima
# ----------------------------------------------------------------------
def resta(a, b): return [a[i] - b[i] for i in range(3)]
def punto3(a, b): return sum(a[i] * b[i] for i in range(3))
def cruz(a, b):
    return [a[1]*b[2] - a[2]*b[1], a[2]*b[0] - a[0]*b[2], a[0]*b[1] - a[1]*b[0]]
def unitario(a):
    l = math.sqrt(punto3(a, a)) or 1.0
    return [c / l for c in a]


def newell(ps):
    """Normal de un polígono (regla de la mano derecha)."""
    nx = ny = nz = 0.0
    for i in range(len(ps)):
        a, b = ps[i], ps[(i + 1) % len(ps)]
        nx += (a[1] - b[1]) * (a[2] + b[2])
        ny += (a[2] - b[2]) * (a[0] + b[0])
        nz += (a[0] - b[0]) * (a[1] + b[1])
    return [nx, ny, nz]


# ----------------------------------------------------------------------
# Primitivas: vértice, cuadrilátero, caja, parche
# ----------------------------------------------------------------------
def vert(p):
    vertices.append([float(c) for c in p])
    return len(vertices)


def quad(m, pts, ref=None, hacia=None):
    """Polígono -> triángulos. Descarta puntos repetidos (un cuadrado que
    degenera en el eje queda como triángulo). El sentido de giro se corrige
    para que la normal mire hacia `hacia`, o alejándose del punto `ref`."""
    lim = []
    for p in pts:
        if not lim or math.dist(p, lim[-1]) > 1e-6:
            lim.append(p)
    if len(lim) > 1 and math.dist(lim[0], lim[-1]) <= 1e-6:
        lim.pop()
    if len(lim) < 3:
        return
    n = newell(lim)
    d = None
    if hacia is not None:
        d = hacia
    elif ref is not None:
        c = [sum(p[k] for p in lim) / len(lim) for k in range(3)]
        d = resta(c, ref)
    if d is not None and punto3(n, d) < 0:
        lim.reverse()
    ids = [vert(p) for p in lim]
    for i in range(1, len(ids) - 1):
        caras.append((m, [ids[0], ids[i], ids[i + 1]]))


def P(a, u, w, y, cx=0.0, cz=0.0):
    """Punto en un marco polar: u = radial, w = tangencial, y = altura.
    Con a = 0 es simplemente (x=u, y, z=w)."""
    c, s = math.cos(a), math.sin(a)
    return (cx + u * c - w * s, y, cz + u * s + w * c)


def caja(m, a, u0, u1, w0, w1, y0, y1, cx=0.0, cz=0.0):
    """Caja en el marco polar de ángulo `a` (a=0: u=x, w=z)."""
    f = lambda u, w, y: P(a, u, w, y, cx, cz)
    ref = f((u0 + u1) / 2, (w0 + w1) / 2, (y0 + y1) / 2)
    for ps in [
        [f(u0, w0, y0), f(u0, w1, y0), f(u0, w1, y1), f(u0, w0, y1)],
        [f(u1, w0, y0), f(u1, w1, y0), f(u1, w1, y1), f(u1, w0, y1)],
        [f(u0, w0, y0), f(u1, w0, y0), f(u1, w0, y1), f(u0, w0, y1)],
        [f(u0, w1, y0), f(u1, w1, y0), f(u1, w1, y1), f(u0, w1, y1)],
        [f(u0, w0, y0), f(u1, w0, y0), f(u1, w1, y0), f(u0, w1, y0)],
        [f(u0, w0, y1), f(u1, w0, y1), f(u1, w1, y1), f(u0, w1, y1)],
    ]:
        quad(m, ps, ref=ref)


def bilineal(ps, u, v):
    return [((1-u)*(1-v)*ps[0][k] + u*(1-v)*ps[1][k]
             + u*v*ps[2][k] + (1-u)*v*ps[3][k]) for k in range(3)]


def parche(m, ps, u0, u1, v0, v1, off):
    """Cuadrado pintado sobre otro (ps en orden (0,0),(1,0),(1,1),(0,1)),
    levantado `off` para que no pelee con la superficie de abajo."""
    pts = [bilineal(ps, u, v) for u, v in [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]]
    pts = [[p[k] + off[k] for k in range(3)] for p in pts]
    quad(m, pts, hacia=off)


def normal_exterior(ps):
    """Normal unitaria de una cara del núcleo, apuntando lejos del eje Y."""
    n = unitario(newell(ps))
    cx = sum(p[0] for p in ps) / len(ps)
    cz = sum(p[2] for p in ps) / len(ps)
    if n[0] * cx + n[2] * cz < 0:
        n = [-c for c in n]
    return n


def ventana(ps, n, e=0.03):
    """Marco + cristal + reflejos (igual que la ventana de la nave)."""
    off = lambda k: [c * e * k for c in n]
    parche('Refuerzos_oscuros', ps, .02, .98, .02, .98, off(1))
    parche('Marco_claro', ps, .07, .93, .07, .93, off(2))
    parche('Vidrio', ps, .11, .89, .11, .89, off(3))
    for u0, u1 in [(.22, .30), (.36, .40)]:
        parche('Reflejo', ps, u0, u1, .13, .87, off(4))


def torno(perfil, segs, cx=0.0, cz=0.0, ang0=0.0, cerrado=False):
    """Superficie de revolución alrededor de un eje vertical.
    perfil = [(radio, altura, material)...]; el material de cada tramo es el
    del punto donde empieza. Recórrelo desde abajo-centro, hacia afuera,
    sube y vuelve hacia el centro por arriba: así las normales salen hacia fuera."""
    n = len(perfil)
    for i in range(n if cerrado else n - 1):
        r0, y0, m = perfil[i]
        r1, y1, _ = perfil[(i + 1) % n]
        for k in range(segs):
            a0 = ang0 + k * math.tau / segs
            a1 = ang0 + (k + 1) * math.tau / segs
            p = lambda r, a, y: (cx + r * math.cos(a), y, cz + r * math.sin(a))
            quad(m, [p(r0, a0, y0), p(r1, a0, y1), p(r1, a1, y1), p(r0, a1, y0)])


def varilla(m, cx, cz, r, y0, y1, punta=None, segs=8):
    """Mástil: prisma delgado con tapas; `punta` = material de la tapa de arriba."""
    torno([(0, y0, m), (r, y0, m), (r, y1, punta or m), (0, y1, punta or m)],
          segs, cx, cz)


# ----------------------------------------------------------------------
# 1. Anillo exterior
# ----------------------------------------------------------------------
SEG_ANILLO = 48
R_IN, R_OUT, Y0, Y1 = 16.0, 20.0, -2.0, 2.0

# Cuerpo del anillo: caras de abajo, exterior, techo e interior.
torno([(R_IN, Y0, 'Casco_gris'), (R_OUT, Y0, 'Casco_gris'),
       (R_OUT, Y1, 'Casco_gris'), (R_IN, Y1, 'Casco_gris')],
      SEG_ANILLO, cerrado=True)
# Reborde exterior claro arriba y pie oscuro abajo.
torno([(19.2, Y1, 'Casco_gris'), (R_OUT, Y1, 'Casco_gris'),
       (R_OUT, Y1 + .4, 'Marco_claro'), (19.2, Y1 + .4, 'Casco_gris')],
      SEG_ANILLO, cerrado=True)
torno([(16.6, Y0 - .5, 'Refuerzos_oscuros'), (19.4, Y0 - .5, 'Refuerzos_oscuros'),
       (19.4, Y0, 'Refuerzos_oscuros'), (16.6, Y0, 'Refuerzos_oscuros')],
      SEG_ANILLO, cerrado=True)

for j in range(SEG_ANILLO):
    a0, a1 = j * math.tau / SEG_ANILLO, (j + 1) * math.tau / SEG_ANILLO
    am = (a0 + a1) / 2
    radial = [math.cos(am), 0.0, math.sin(am)]

    # --- pared exterior: placas, con una faceta de unión cada 4 ---
    pared = [P(a0, R_OUT, 0, Y0), P(a1, R_OUT, 0, Y0), P(a1, R_OUT, 0, Y1), P(a0, R_OUT, 0, Y1)]
    off = lambda d: [c * d for c in radial]
    if j % 4 != 3:
        for v0, v1 in [(.06, .47), (.53, .94)]:
            parche('Refuerzos_oscuros', pared, .05, .95, v0, v1, off(.05))
            parche('Placa', pared, .10, .90, v0 + .06, v1 - .06, off(.10))
    else:
        parche('Refuerzos_oscuros', pared, .0, 1.0, .0, 1.0, off(.05))
        if j % 12 == 3:   # franja cian vertical cada tres módulos
            parche('Luz_cian', pared, .38, .62, .12, .88, off(.10))

    # --- techo: costuras oscuras entre módulos ---
    if j % 4 == 3:
        techo = [P(a0, R_IN, 0, Y1), P(a0, 19.2, 0, Y1), P(a1, 19.2, 0, Y1), P(a1, R_IN, 0, Y1)]
        parche('Refuerzos_oscuros', techo, .02, .98, .30, .70, [0, .04, 0])

    # --- pared interior: banda cian con marco oscuro (como la de la imagen) ---
    inte = [P(a0, R_IN, 0, Y0), P(a1, R_IN, 0, Y0), P(a1, R_IN, 0, Y1), P(a0, R_IN, 0, Y1)]
    hacia_dentro = [-c * 0.04 for c in radial]
    parche('Refuerzos_oscuros', inte, 0, 1, .66, .96, hacia_dentro)
    parche('Luz_cian', inte, 0, 1, .735, .885, [c * 1.6 for c in hacia_dentro])

# Cuatro balizas cuadradas en el techo del anillo (diagonales).
for k in range(4):
    a = math.radians(45 + 90 * k)
    caja('Refuerzos_oscuros', a, 17.1, 18.9, -.9, .9, Y1, Y1 + .3)
    caja('Marco_claro', a, 17.4, 18.6, -.6, .6, Y1 + .3, Y1 + .4)
    caja('Luz_cian', a, 17.65, 18.35, -.35, .35, Y1 + .4, Y1 + .45)

# Propulsor en el costado (a 180°, entre dos brazos solares).
a = math.pi
caja('Refuerzos_oscuros', a, 19.9, 21.7, -1.3, 1.3, -1.2, 1.2)
caja('Marco_claro', a, 21.7, 21.85, -.95, .95, -.85, .85)
caja('Luz_cian', a, 21.85, 21.95, -.6, .6, -.5, .5)
for w in (-.8, .8):
    caja('Refuerzos_oscuros', a, 21.7, 25.5, w - .07, w + .07, -.67, -.53)


# ----------------------------------------------------------------------
# 2. Núcleo central (prisma de 12 lados) + tallo inferior
# ----------------------------------------------------------------------
SEG_HUB, ANG0_HUB = 12, math.radians(-15)   # caras centradas en 0°, 30°, 60°...


def anillo_hub(r, y):
    return [(r * math.cos(ANG0_HUB + k * math.tau / SEG_HUB), y,
             r * math.sin(ANG0_HUB + k * math.tau / SEG_HUB)) for k in range(SEG_HUB + 1)]


G, O, M, PL = 'Casco_gris', 'Refuerzos_oscuros', 'Marco_claro', 'Placa'
torno([
    (0.0, -5.4, O), (2.8, -5.4, O),      # fondo
    (5.6, -3.6, O),                      # cono inferior
    (6.5, -3.0, G), (6.5, 3.4, G),       # chaflán y tambor
    (6.65, 3.4, O), (6.65, 3.9, O),      # cinturón oscuro
    (6.5, 3.9, O),
    (5.7, 7.5, O),                       # banda de ventanas (se inclina hacia dentro)
    (5.95, 7.5, O), (5.95, 8.0, M),      # cornisa clara
    (4.3, 9.4, G), (3.6, 9.4, G),        # techo inclinado y repisa
    (3.6, 10.3, G), (0.0, 10.3, PL),     # segundo escalón y cubierta
], SEG_HUB, ang0=ANG0_HUB)
# (el último punto solo marca el final; su material no se usa)

# Placas del tambor; en una de cada tres caras, tres barras cian.
A, B = anillo_hub(6.5, -2.6), anillo_hub(6.5, 3.0)
for k in range(SEG_HUB):
    ps = [A[k], A[k + 1], B[k + 1], B[k]]
    n = normal_exterior(ps)
    off = lambda d: [c * d for c in n]
    parche('Refuerzos_oscuros', ps, .07, .93, .04, .96, off(.04))
    parche('Placa', ps, .12, .88, .09, .91, off(.08))
    if k % 3 == 1:
        for u0, u1 in [(.36, .43), (.465, .535), (.57, .64)]:
            parche('Luz_cian', ps, u0, u1, .20, .46, off(.12))

# Ventanas: una por cada cara de la banda inclinada.
A, B = anillo_hub(6.5, 3.9), anillo_hub(5.7, 7.5)
for k in range(SEG_HUB):
    ps = [A[k], A[k + 1], B[k + 1], B[k]]
    ventana(ps, normal_exterior(ps))

# Tallo inferior con collar y antena fina con punta cian.
varilla(O, 0, 0, 1.0, -11.5, -5.4)
torno([(1.0, -8.3, G), (1.7, -8.3, G), (1.7, -7.5, G), (1.0, -7.5, G)], 12, cerrado=True)
varilla(M, 0, 0, .18, -14.5, -11.5)
torno([(0, -14.9, 'Luz_cian'), (.22, -14.5, 'Luz_cian')], 8)


# ----------------------------------------------------------------------
# 3. Parte alta: torre, mástiles y platos de antena
# ----------------------------------------------------------------------
caja(O, 0, -1.5, 1.5, -1.2, 1.2, 10.3, 14.2)
caja(M, 0, -1.7, 1.7, -1.4, 1.4, 14.2, 14.5)
caja(G, 0, 1.5, 2.4, -.6, .6, 10.3, 12.2)       # módulo lateral
caja(G, 0, -2.3, -1.5, -.5, .5, 10.3, 11.6)
caja('Luz_cian', 0, 1.0, 1.3, 1.2, 1.26, 12.0, 13.4)   # ranura luminosa

varilla(M, .7, .5, .20, 14.5, 23.0, 'Luz_cian')
varilla(M, -.7, -.3, .16, 14.5, 20.0, 'Luz_cian')
varilla(M, 1.1, -.5, .12, 14.5, 18.0, 'Luz_cian')
varilla(M, -1.2, .6, .12, 14.5, 17.0, 'Luz_cian')


def plato(th, radio=1.5, elev=math.radians(50)):
    """Plato de antena sobre la repisa del techo, mirando afuera y arriba."""
    rad = [math.cos(th), 0.0, math.sin(th)]
    cx, cz = 3.9 * rad[0], 3.9 * rad[2]
    c = [cx, 11.6, cz]
    varilla(O, cx, cz, .15, 9.4, 11.3)
    a = unitario([rad[0] * math.cos(elev), math.sin(elev), rad[2] * math.cos(elev)])
    t1 = unitario(cruz([0, 1, 0], a))
    t2 = cruz(a, t1)
    n = 16
    borde = [[c[i] + radio * (math.cos(k * math.tau / n) * t1[i] + math.sin(k * math.tau / n) * t2[i])
              for i in range(3)] for k in range(n + 1)]
    apice = [c[i] - a[i] * .55 for i in range(3)]
    trasera = [c[i] - a[i] * .70 for i in range(3)]
    for k in range(n):
        quad(M, [apice, borde[k], borde[k + 1]], hacia=a)                     # cara cóncava
        quad(G, [trasera, borde[k], borde[k + 1]], hacia=[-x for x in a])     # envés
    # alimentador al frente del plato
    punta = [c[i] + a[i] * .9 for i in range(3)]
    quad('Luz_cian', [punta, [punta[0] + .12, punta[1], punta[2]],
                      [punta[0], punta[1] + .12, punta[2]]], hacia=a)


for grados in (20, 140, 260):
    plato(math.radians(grados))


# ----------------------------------------------------------------------
# 4. Brazos de módulos (ejes) que unen el núcleo con el anillo
# ----------------------------------------------------------------------
for k in range(4):
    a = k * math.pi / 2
    caja(O, a, 6.3, 16.3, -.9, .9, -.7, .5)                   # conducto central
    for i in range(5):
        u0 = 7.0 + i * 1.75
        u1 = u0 + 1.55
        hw = 2.0 - .25 * i                                     # se afinan hacia el anillo
        top = 2.7 - .3 * i
        caja(G, a, u0, u1, -hw, hw, -1.3, top)
        caja(O, a, u0 - .02, u1 + .02, -hw - .02, hw + .02, -1.3, -.95)
        ps = [P(a, u0, -hw, top), P(a, u1, -hw, top), P(a, u1, hw, top), P(a, u0, hw, top)]
        parche(O, ps, .06, .94, .06, .94, [0, .02, 0])
        parche(PL, ps, .12, .88, .12, .88, [0, .05, 0])
        if i % 2 == 0:
            caja('Luz_cian', a, u0 + .5, u1 - .5, -.18, .18, top + .06, top + .10)


# ----------------------------------------------------------------------
# 5. Brazos de paneles solares (diagonales): 2 x 2 paneles por brazo
# ----------------------------------------------------------------------
def panel(a, u0, u1, w0, w1, y, nu=6, nw=5):
    caja(O, a, u0, u1, w0, w1, y - .14, y)                    # bastidor oscuro
    for i in range(nu):
        for j in range(nw):
            ua, ub = u0 + (u1 - u0) * (i + .1) / nu, u0 + (u1 - u0) * (i + .9) / nu
            wa, wb = w0 + (w1 - w0) * (j + .1) / nw, w0 + (w1 - w0) * (j + .9) / nw
            quad('Celda_clara' if (i + j) % 4 == 0 else 'Panel_solar_azul',
                 [P(a, ua, wa, y + .03), P(a, ub, wa, y + .03),
                  P(a, ub, wb, y + .03), P(a, ua, wb, y + .03)], hacia=[0, 1, 0])
    t = .22
    for (ua, ub, wa, wb) in [(u0, u1, w0, w0 + t), (u0, u1, w1 - t, w1),
                             (u0, u0 + t, w0, w1), (u1 - t, u1, w0, w1)]:
        caja(M, a, ua, ub, wa, wb, y - .14, y + .10)


Y_PANEL = .2
for k in range(4):
    a = math.radians(45 + 90 * k)
    caja(O, a, 19.9, 22.3, -.9, .9, -1.2, 1.2)                 # soporte en el casco
    caja(O, a, 22.0, 40.2, -.4, .4, -.35, .15)                 # larguero central
    for (u0, u1) in [(22.6, 30.6), (31.6, 40.0)]:
        for (w0, w1) in [(.7, 6.7), (-6.7, -.7)]:
            panel(a, u0, u1, w0, w1, Y_PANEL)
    for (w0, w1) in [(.7, 6.7), (-6.7, -.7)]:
        caja('Luz_cian', a, 40.0, 40.14, w0, w1, Y_PANEL - .08, Y_PANEL + .06)


# ----------------------------------------------------------------------
# Escritura (mismo formato que nave.obj / nave.mtl)
# ----------------------------------------------------------------------
COLORES = {
    'Casco_gris': (.43, .48, .51), 'Refuerzos_oscuros': (.08, .13, .16),
    'Marco_claro': (.52, .62, .65), 'Placa': (.38, .43, .47),
    'Panel_solar_azul': (.045, .105, .29), 'Celda_clara': (.075, .16, .36),
    'Vidrio': (.25, .67, .77), 'Reflejo': (.64, .91, .94),
    'Fondo_de_toberas': (.24, .91, .94), 'Luz_cian': (.24, .94, .96),
}
salida = ['# Estación espacial triangulada\nmtllib estacion.mtl\no Estacion\n']
salida += ['v %.6f %.6f %.6f\n' % tuple(v) for v in vertices]
for m in COLORES:
    salida.append(f'usemtl {m}\n')
    for mat, ids in caras:
        if mat == m:
            salida.append(f'f {ids[0]} {ids[1]} {ids[2]}\n')
(CARPETA / 'estacion.obj').write_text(''.join(salida))
(CARPETA / 'estacion.mtl').write_text(''.join(
    f'newmtl {m}\nKd {r} {g} {b}\n\n' for m, (r, g, b) in COLORES.items()))
print(f'Estación: {len(vertices)} vértices, {len(caras)} triángulos')
