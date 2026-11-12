"""Genera personaje.obj y personaje.mtl: un muñeco estilo caricatura / Minecraft.

Uso: python3 crear_personaje.py [carpeta_salida]

Basado en la foto de referencia: impermeable amarillo abierto con botones,
gorro azul marino de punto, suéter azul con franjas gris/blanca/roja y cierre,
camiseta blanca, anteojos redondos, barba corta y expresión de asombro cansado.

Mismo estilo que crear_tars.py: solo 'v', 'usemtl' y 'f' triangulares,
compatible con obj.rs / estacion.rs de tu proyecto 3 (sin UV, sin texturas).

Marco del modelo (alto total ~1.09, mucho más pequeño que el TARS):
    x = derecha de quien lo mira de frente
    y = arriba, 0 = suelo
    z = hacia el frente (la cara)
Centrado en x = 0, z = 0.
"""
import math
import sys
from pathlib import Path

SALIDA = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent

COLORES = {
    'Impermeable':    (0.93, 0.74, 0.10),
    'Impermeable_osc': (0.74, 0.55, 0.05),
    'Gorro':          (0.08, 0.13, 0.27),
    'Gorro_banda':    (0.05, 0.09, 0.20),
    'Jersey':         (0.07, 0.10, 0.21),
    'Raya_gris':      (0.62, 0.64, 0.67),
    'Raya_blanca':    (0.92, 0.92, 0.92),
    'Raya_roja':      (0.76, 0.14, 0.17),
    'Camiseta':       (0.93, 0.93, 0.90),
    'Punos_oliva':    (0.38, 0.38, 0.17),
    'Piel':           (0.95, 0.77, 0.62),
    'Piel_osc':       (0.86, 0.64, 0.50),
    'Barba':          (0.50, 0.38, 0.30),
    'Pelo':           (0.40, 0.26, 0.17),
    'Ojo_blanco':     (0.97, 0.97, 0.97),
    'Ojo_iris':       (0.24, 0.12, 0.30),
    'Lentes':         (0.80, 0.62, 0.22),
    'Boca':           (0.30, 0.10, 0.15),
    'Boton':          (0.38, 0.22, 0.10),
    'Metal':          (0.82, 0.84, 0.88),
    'Periodico':      (0.88, 0.88, 0.85),
    'Tinta':          (0.38, 0.38, 0.40),
    'Pantalon':       (0.17, 0.19, 0.24),
    'Zapato':         (0.12, 0.10, 0.10),
}
V = []
F = {m: [] for m in COLORES}


# -------------------------------------------------------------- utilidades
def sub(a, b): return [a[i] - b[i] for i in range(3)]
def dot(a, b): return sum(a[i] * b[i] for i in range(3))
def cruz(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]


def poligono(m, pts, centro):
    """Polígono plano; la normal se orienta para alejarse de `centro`."""
    ids = []
    for p in pts:
        V.append(tuple(p))
        ids.append(len(V))
    a, b, c = (V[i - 1] for i in ids[:3])
    n = cruz(sub(b, a), sub(c, a))
    medio = [sum(V[i - 1][k] for i in ids) / len(ids) for k in range(3)]
    if dot(n, sub(medio, centro)) < 0:
        ids.reverse()
    for i in range(1, len(ids) - 1):
        F[m].append((ids[0], ids[i], ids[i + 1]))


def caja(m, x0, x1, y0, y1, z0, z1):
    centro = ((x0+x1)/2, (y0+y1)/2, (z0+z1)/2)
    a, b, c, d = (x0,y0,z0), (x1,y0,z0), (x1,y1,z0), (x0,y1,z0)
    e, f, g, h = (x0,y0,z1), (x1,y0,z1), (x1,y1,z1), (x0,y1,z1)
    for ps in [(a,d,c,b), (e,f,g,h), (a,e,h,d), (b,c,g,f), (d,h,g,c), (a,b,f,e)]:
        poligono(m, ps, centro)


def octogono(cx, cz, hw, hd, b):
    return [(cx-hw+b, cz-hd), (cx+hw-b, cz-hd), (cx+hw, cz-hd+b), (cx+hw, cz+hd-b),
            (cx+hw-b, cz+hd), (cx-hw+b, cz+hd), (cx-hw, cz+hd-b), (cx-hw, cz-hd+b)]


def prisma(m, contorno, y0, y1):
    n = len(contorno)
    centro = (sum(p[0] for p in contorno)/n, (y0+y1)/2, sum(p[1] for p in contorno)/n)
    for i in range(n):
        (px, pz), (qx, qz) = contorno[i], contorno[(i+1) % n]
        poligono(m, [(px,y0,pz), (qx,y0,qz), (qx,y1,qz), (px,y1,pz)], centro)
    poligono(m, [(x, y0, z) for x, z in contorno], centro)
    poligono(m, [(x, y1, z) for x, z in contorno], centro)


def anillo_z(m, cx, cy, r_ext, r_int, z0, z1, n=14):
    """Aro (anteojos) con eje paralelo a Z. No es convexo: cada cara se orienta aparte."""
    for k in range(n):
        a0, a1 = k*math.tau/n, (k+1)*math.tau/n
        am = (a0 + a1) / 2
        dx, dy = math.cos(am), math.sin(am)
        e0 = (cx+r_ext*math.cos(a0), cy+r_ext*math.sin(a0)); e1 = (cx+r_ext*math.cos(a1), cy+r_ext*math.sin(a1))
        i0 = (cx+r_int*math.cos(a0), cy+r_int*math.sin(a0)); i1 = (cx+r_int*math.cos(a1), cy+r_int*math.sin(a1))
        zm = (z0 + z1) / 2
        poligono(m, [(*e0, z0), (*e1, z0), (*e1, z1), (*e0, z1)], (cx, cy, zm))      # pared exterior
        poligono(m, [(*i0, z0), (*i1, z0), (*i1, z1), (*i0, z1)],
                 (cx+3*dx, cy+3*dy, zm))                                              # pared interior
        poligono(m, [(*i0, z1), (*i1, z1), (*e1, z1), (*e0, z1)], (cx, cy, z1-1))    # cara frontal
        poligono(m, [(*i0, z0), (*i1, z0), (*e1, z0), (*e0, z0)], (cx, cy, z0+1))    # cara trasera


# ----------------------------------------------------------------- piezas
def piernas():
    for lado in (-1, 1):
        x = lado * 0.075
        caja('Pantalon', x-0.055, x+0.055, 0.04, 0.26, -0.06, 0.06)
        caja('Zapato', x-0.062, x+0.062, 0.0, 0.045, -0.07, 0.10)


def torso():
    # Suéter visible por el frente abierto del impermeable.
    caja('Jersey', -0.10, 0.10, 0.24, 0.63, -0.085, 0.085)
    z = 0.085
    for mat, y0, y1 in [('Raya_gris', 0.52, 0.555), ('Raya_blanca', 0.485, 0.52),
                        ('Raya_roja', 0.45, 0.485)]:
        caja(mat, -0.058, 0.058, y0, y1, z, z + 0.006)
    caja('Metal', -0.004, 0.004, 0.555, 0.64, z, z + 0.005)        # cierre
    caja('Metal', -0.010, 0.010, 0.585, 0.615, z, z + 0.012)       # tirador
    # Cuello alto del suéter y camiseta blanca asomando.
    caja('Jersey', -0.075, 0.075, 0.62, 0.665, -0.075, 0.075)
    caja('Camiseta', -0.045, 0.045, 0.60, 0.67, 0.075, 0.082)
    # Impermeable: dos paneles delanteros, espalda y bolsillos.
    for lado in (-1, 1):
        x0, x1 = (0.06, 0.17) if lado > 0 else (-0.17, -0.06)
        caja('Impermeable', x0, x1, 0.20, 0.62, -0.085, 0.10)
        px0, px1 = (0.08, 0.15) if lado > 0 else (-0.15, -0.08)
        caja('Impermeable_osc', px0, px1, 0.27, 0.30, 0.10, 0.108)   # solapa de bolsillo
        # Cuello levantado.
        cx0, cx1 = (0.05, 0.115) if lado > 0 else (-0.115, -0.05)
        caja('Impermeable', cx0, cx1, 0.60, 0.69, -0.09, 0.10)
    caja('Impermeable', -0.17, 0.17, 0.20, 0.62, -0.10, -0.085)
    caja('Impermeable', -0.115, 0.115, 0.60, 0.69, -0.105, -0.09)
    # Botones al borde del panel izquierdo (el del periódico).
    for y in (0.30, 0.40, 0.50):
        caja('Boton', -0.092, -0.074, y, y + 0.018, 0.10, 0.108)
    # Costura interior oscura del borde de ambos paneles.
    for x in (-0.062, 0.058):
        caja('Impermeable_osc', x, x + 0.004, 0.20, 0.62, 0.10, 0.104)


def brazos():
    # Brazo izquierdo del personaje (x positivo): cuelga.
    caja('Impermeable', 0.18, 0.255, 0.285, 0.61, -0.04, 0.04)
    caja('Punos_oliva', 0.18, 0.255, 0.25, 0.285, -0.04, 0.04)
    caja('Piel', 0.185, 0.25, 0.17, 0.25, -0.035, 0.035)
    # Brazo derecho (x negativo): cuelga, simétrico.
    caja('Impermeable', -0.255, -0.18, 0.285, 0.61, -0.04, 0.04)
    caja('Punos_oliva', -0.255, -0.18, 0.25, 0.285, -0.04, 0.04)
    caja('Piel', -0.25, -0.185, 0.17, 0.25, -0.035, 0.035)


def cabeza():
    caja('Piel', -0.04, 0.04, 0.62, 0.68, -0.04, 0.04)                    # cuello
    prisma('Piel', octogono(0, 0, 0.17, 0.15, 0.05), 0.66, 0.98)          # cráneo
    zf = 0.15
    # Barba corta: cubre la mandíbula, patillas y bigote.
    prisma('Barba', octogono(0, 0, 0.173, 0.153, 0.05), 0.66, 0.745)
    for lado in (-1, 1):
        x = lado * 0.171
        caja('Barba', min(x, x + lado*0.004), max(x, x + lado*0.004), 0.745, 0.88, 0.03, 0.12)
    caja('Barba', -0.055, 0.055, 0.752, 0.778, zf, zf + 0.008)            # bigote
    # Boca entreabierta: asombro cansado.
    caja('Boca', -0.016, 0.016, 0.713, 0.735, zf + 0.003, zf + 0.009)
    caja('Boca', -0.024, 0.024, 0.719, 0.729, zf + 0.003, zf + 0.009)
    # Nariz y orejas.
    caja('Piel_osc', -0.018, 0.018, 0.775, 0.815, zf, zf + 0.03)
    for lado in (-1, 1):
        x0, x1 = (0.17, 0.195) if lado > 0 else (-0.195, -0.17)
        caja('Piel', x0, x1, 0.79, 0.86, -0.06, 0.01)
    # Ojos muy abiertos mirando hacia arriba, ojeras y cejas alzadas e inclinadas
    # hacia adentro (preocupación / intriga).
    for lado in (-1, 1):
        cx = lado * 0.065
        caja('Ojo_blanco', cx-0.028, cx+0.028, 0.800, 0.842, zf, zf + 0.004)
        caja('Ojo_iris', cx-0.015, cx+0.015, 0.816, 0.842, zf + 0.004, zf + 0.008)
        caja('Piel_osc', cx-0.034, cx+0.034, 0.758, 0.770, zf + 0.002, zf + 0.006)    # ojera
        ext = sorted((cx + lado*0.040, cx + lado*0.0))      # tramo exterior (más bajo)
        inn = sorted((cx, cx - lado*0.040))                 # tramo interior (más alto)
        caja('Pelo', ext[0], ext[1], 0.888, 0.902, zf + 0.002, zf + 0.009)
        caja('Pelo', inn[0], inn[1], 0.903, 0.917, zf + 0.002, zf + 0.009)
        anillo_z('Lentes', cx, 0.82, 0.052, 0.042, zf + 0.01, zf + 0.02)
        x = lado * 0.173
        caja('Lentes', min(x, x + lado*0.005), max(x, x + lado*0.005), 0.815, 0.827, 0.0, 0.13)
    caja('Lentes', -0.016, 0.016, 0.822, 0.834, zf + 0.01, zf + 0.02)     # puente
    caja('Pelo', -0.10, 0.10, 0.922, 0.93, zf, zf + 0.006)                # flequillo


def gorro():
    prisma('Gorro_banda', octogono(0, 0, 0.184, 0.164, 0.055), 0.925, 0.98)
    prisma('Gorro', octogono(0, 0, 0.176, 0.156, 0.065), 0.98, 1.07)
    prisma('Gorro', octogono(0, 0, 0.12, 0.10, 0.04), 1.07, 1.095)
    # Costuras verticales de punto sobre la banda.
    for k in range(-2, 3):
        caja('Gorro', k*0.05 - 0.004, k*0.05 + 0.004, 0.925, 0.98, 0.164, 0.168)


piernas()
torso()
brazos()
cabeza()
gorro()

# ---------------------------------------------------------------- escritura
salida = ['# Personaje caricatura; generado por crear_personaje.py\n', 'mtllib personaje.mtl\n']
salida += ['v %.6f %.6f %.6f\n' % v for v in V]
total = 0
for m in COLORES:
    if not F[m]:
        continue
    salida.append(f'usemtl {m}\n')
    salida += [f'f {a} {b} {c}\n' for a, b, c in F[m]]
    total += len(F[m])
(SALIDA / 'personaje.obj').write_text(''.join(salida))
(SALIDA / 'personaje.mtl').write_text(''.join(
    f'newmtl {m}\nKd {r} {g} {b}\n\n' for m, (r, g, b) in COLORES.items()))
print(f'Personaje: {len(V)} vértices, {total} triángulos -> {SALIDA}')
