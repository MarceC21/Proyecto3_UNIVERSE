"""Genera rocky.obj y rocky.mtl: Rocky (Project Hail Mary), criatura de roca.

Uso: python3 crear_rocky.py [carpeta_salida] [--brazos]
  Sin opciones: las cinco extremidades apoyadas en el suelo -> rocky.obj
  --brazos:     dos extremidades levantadas como brazos -> rocky_brazos.obj

Cuerpo de caparazón ovoide (más largo hacia abajo), cinco extremidades en simetría pentagonal
(las dos delanteras son brazos con mano de tres dedos; las otras tres son
patas con pie de pala plana), manchas verdes y grietas.

Mismo estilo que crear_tars.py / crear_personaje.py: solo 'v', 'usemtl' y 'f'
triangulares, sin UV. El efecto de roca texturizada se logra repartiendo las
caras entre tres tonos de café (Roca, Roca_clara, Roca_oscura) con una semilla
fija, así que el resultado es reproducible.

Marco del modelo (alto ~1.06 sin contar los brazos levantados):
    x = derecha de quien lo mira de frente
    y = arriba, 0 = suelo
    z = hacia el frente (de donde salen los brazos)
Centrado en x = 0, z = 0.
"""
import math
import random
import sys
from pathlib import Path

ARGS = [a for a in sys.argv[1:] if not a.startswith('--')]
BRAZOS = '--brazos' in sys.argv        # variante con dos brazos levantados
NOMBRE = 'rocky_brazos' if BRAZOS else 'rocky'
SALIDA = Path(ARGS[0]) if ARGS else Path(__file__).resolve().parent
RNG = random.Random(2026)

COLORES = {
    'Roca':         (0.55, 0.39, 0.27),
    'Roca_clara':   (0.67, 0.50, 0.36),
    'Roca_oscura':  (0.37, 0.26, 0.18),
    'Grieta':       (0.17, 0.11, 0.08),
    'Mancha_verde': (0.14, 0.86, 0.46),
}
V = []
F = {m: [] for m in COLORES}


# -------------------------------------------------------------- utilidades
def sub(a, b): return [a[i] - b[i] for i in range(3)]
def add(a, b): return [a[i] + b[i] for i in range(3)]
def mul(a, k): return [a[i] * k for i in range(3)]
def dot(a, b): return sum(a[i] * b[i] for i in range(3))
def cruz(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
def norm(a):
    l = math.sqrt(dot(a, a))
    return [x / l for x in a] if l > 1e-9 else a


def roca():
    """Tono de roca al azar (manchas de color como si fuera una textura)."""
    r = RNG.random()
    return 'Roca' if r < 0.50 else 'Roca_clara' if r < 0.78 else 'Roca_oscura'


def poligono(m, pts, centro):
    """Polígono (casi) plano; la normal se orienta para alejarse de `centro`."""
    limpio = []
    for p in pts:
        if not limpio or max(abs(p[k] - limpio[-1][k]) for k in range(3)) > 1e-7:
            limpio.append(p)
    if len(limpio) > 1 and max(abs(limpio[0][k] - limpio[-1][k]) for k in range(3)) <= 1e-7:
        limpio.pop()
    if len(limpio) < 3:
        return
    ids = []
    for p in limpio:
        V.append(tuple(p))
        ids.append(len(V))
    a, b, c = (V[i - 1] for i in ids[:3])
    n = cruz(sub(b, a), sub(c, a))
    medio = [sum(V[i - 1][k] for i in ids) / len(ids) for k in range(3)]
    if dot(n, sub(medio, centro)) < 0:
        ids.reverse()
    for i in range(1, len(ids) - 1):
        F[m].append((ids[0], ids[i], ids[i + 1]))


def segmento(mat, p0, p1, a0, b0, a1, b1, u_hint, n=6, jit=0.0):
    """Prisma cónico entre p0 y p1 con sección elíptica (a = eje u, b = eje v).
    mat puede ser un nombre o una función sin argumentos (para variar por cara)."""
    eje = norm(sub(p1, p0))
    u = norm(sub(u_hint, mul(eje, dot(u_hint, eje))))
    v = cruz(eje, u)
    fase = RNG.random() * math.tau / n
    anillos = []
    for p, a, b in ((p0, a0, b0), (p1, a1, b1)):
        r = []
        for k in range(n):
            t = fase + k * math.tau / n
            f = 1 + RNG.uniform(-jit, jit)
            r.append(add(p, add(mul(u, a * f * math.cos(t)), mul(v, b * f * math.sin(t)))))
        anillos.append(r)
    centro = mul(add(p0, p1), 0.5)
    pick = mat if callable(mat) else (lambda: mat)
    for k in range(n):
        k2 = (k + 1) % n
        poligono(pick(), [anillos[0][k], anillos[0][k2], anillos[1][k2], anillos[1][k]], centro)
    poligono(pick(), anillos[0], centro)
    poligono(pick(), anillos[1], centro)


# ------------------------------------------------------------------ cuerpo
CENTRO = [0.0, 0.72, 0.0]
RX, RZ = 0.47, 0.44
RY_ARR, RY_ABA = 0.34, 0.56             # ovoide: se alarga hacia abajo
PIVOTES = {}                            # articulaciones, para animar después


def punto_cuerpo(d, extra=0.0):
    """Punto sobre el ovoide del caparazón en la dirección unitaria d."""
    ry = RY_ARR if d[1] >= 0 else RY_ABA
    p = [CENTRO[0] + d[0]*RX, CENTRO[1] + d[1]*ry, CENTRO[2] + d[2]*RZ]
    n = norm([d[0]/RX, d[1]/ry, d[2]/RZ])
    return add(p, mul(n, extra)), n


def caparazon():
    NLAT, NLON = 8, 12
    malla = {}
    for j in range(NLAT + 1):
        lat = -math.pi/2 + j * math.pi / NLAT
        for i in range(NLON):
            lon = i * math.tau / NLON
            ruido = 1 + (0.0 if j in (0, NLAT) else RNG.uniform(-0.04, 0.04))
            ry = RY_ARR if lat >= 0 else RY_ABA
            x = RX * math.cos(lat) * math.cos(lon) * ruido
            y = ry * math.sin(lat) * ruido
            z = RZ * math.cos(lat) * math.sin(lon) * ruido
            malla[(j, i)] = [CENTRO[0] + x, CENTRO[1] + y, CENTRO[2] + z]
    for j in range(NLAT):
        for i in range(NLON):
            i2 = (i + 1) % NLON
            poligono(roca(), [malla[(j, i)], malla[(j, i2)], malla[(j+1, i2)], malla[(j+1, i)]], CENTRO)


def mancha(punto, normal, radio=0.05):
    """Disco verde algo saliente, orientado según la normal de la superficie."""
    a = sub(punto, mul(normal, 0.015))
    b = add(punto, mul(normal, 0.012))
    hint = [0, 1, 0] if abs(normal[1]) < 0.9 else [1, 0, 0]
    segmento('Mancha_verde', a, b, radio, radio, radio * 0.8, radio * 0.8, hint, n=8)


def grieta(punto, normal, largo=0.2):
    hint = [0, 1, 0] if abs(normal[1]) < 0.9 else [1, 0, 0]
    t = norm(cruz(normal, hint))
    t = norm(add(mul(t, RNG.uniform(-1, 1)), mul(cruz(normal, t), RNG.uniform(-1, 1))))
    a = sub(punto, mul(t, largo / 2))
    b = add(punto, mul(t, largo / 2))
    segmento('Grieta', a, b, 0.006, 0.006, 0.006, 0.006, normal, n=4)


# -------------------------------------------------------------- extremidades
def datos_limite(ang):
    a = math.radians(ang)
    d = [math.sin(a), 0.0, math.cos(a)]          # radial horizontal
    t = [math.cos(a), 0.0, -math.sin(a)]         # tangente horizontal
    return d, t


def punto_radial(d, r, y):
    return [CENTRO[0] + d[0]*r, y, CENTRO[2] + d[2]*r]


def pata(ang):
    d, t = datos_limite(ang)
    cadera = punto_radial(d, 0.30, 0.66)
    rodilla = punto_radial(d, 0.68, 0.92)
    tobillo = punto_radial(d, 0.84, 0.16)
    suelo = punto_radial(d, 0.84, 0.0)
    PIVOTES[f'pata_{ang}'] = {'cadera': cadera, 'rodilla': rodilla, 'tobillo': tobillo}
    # Muslo grueso hacia afuera y arriba; pantorrilla plana; pie vertical que
    # termina exactamente en y = 0 (queda apoyado, sin flotar).
    segmento(roca, cadera, rodilla, 0.15, 0.14, 0.14, 0.13, t, n=6, jit=0.05)
    segmento(roca, rodilla, tobillo, 0.15, 0.10, 0.135, 0.085, t, n=6, jit=0.05)
    segmento(roca, tobillo, suelo, 0.135, 0.09, 0.13, 0.095, t, n=6, jit=0.03)
    n = norm([d[0]*0.4, 1.0, d[2]*0.4])
    mancha(add(rodilla, mul(n, 0.12)), n, 0.05)
    grieta(add(punto_radial(d, 0.84, 0.08), mul(d, 0.085)), d, 0.12)


def mano(muneca, arriba, lado):
    """Palma + tres dedos: dos largos en V y un pulgar corto hacia afuera."""
    sup = add(muneca, mul(arriba, 0.10))
    segmento(roca, muneca, sup, 0.10, 0.06, 0.085, 0.05, lado, n=6, jit=0.04)
    base = sup
    for k, (apertura, largo) in enumerate([(-0.55, 0.28), (0.55, 0.28), (1.35, 0.15)]):
        dedo = norm(add(arriba, mul(lado, apertura)))
        punta = add(base, mul(dedo, largo))
        segmento(roca, base, punta, 0.034, 0.028, 0.02, 0.018, lado, n=5, jit=0.03)


def brazo(ang):
    d, t = datos_limite(ang)
    lado_x = 1 if d[0] > 0 else -1
    hombro = punto_radial(d, 0.34, 0.84)
    codo = punto_radial(d, 0.74, 1.02)
    muneca = add(punto_radial(d, 0.64, 1.55), [-lado_x * 0.08, 0, 0.0])
    PIVOTES[f'brazo_{ang}'] = {'hombro': hombro, 'codo': codo, 'muneca': muneca}
    segmento(roca, hombro, codo, 0.15, 0.13, 0.135, 0.115, t, n=6, jit=0.05)
    segmento(roca, codo, muneca, 0.14, 0.10, 0.11, 0.085, t, n=6, jit=0.05)
    arriba = norm(sub(muneca, codo))
    mano(muneca, arriba, [lado_x, 0.0, 0.0])
    n = norm([d[0]*0.5, 1.0, d[2]*0.5])
    mancha(add(codo, mul(n, 0.12)), n, 0.05)


def construir():
    caparazon()
    # Placa frontal grande, como el escudo del caparazón de la referencia.
    d = norm([0.0, 0.35, 0.93])
    pc, n = punto_cuerpo(d, 0.0)
    segmento(['Roca_clara'][0], sub(pc, mul(n, 0.08)), add(pc, mul(n, 0.045)),
             0.27, 0.20, 0.22, 0.16, [1, 0, 0], n=8, jit=0.04)
    # Manchas y grietas sobre el caparazón.
    for dd in [(-0.40, 0.90, 0.25), (0.55, 0.75, -0.25), (-0.92, 0.30, -0.25),
               (0.88, 0.15, 0.45), (0.05, 0.25, -1.0), (-0.45, 0.5, 0.75)]:
        p, nn = punto_cuerpo(norm(list(dd)), 0.014)
        mancha(p, nn, 0.055)
    for dd in [(0.3, 0.95, 0.1), (-0.7, 0.6, 0.3), (0.6, 0.55, -0.6), (-0.3, 0.2, -0.95),
               (0.9, 0.2, -0.2), (-0.9, 0.1, 0.1), (0.15, 0.9, -0.4), (0.4, 0.4, 0.8)]:
        p, nn = punto_cuerpo(norm(list(dd)), 0.012)
        grieta(p, nn, RNG.uniform(0.14, 0.26))
    # Cinco extremidades a 72°. Los brazos son las dos laterales (±72°) y las
    # patas son una delantera (0°) y dos traseras (±144°): los pies forman un
    # triángulo que contiene el centro del cuerpo, así que con los brazos
    # arriba sigue bien apoyado.
    if BRAZOS:
        for ang in (72, -72):
            brazo(ang)
        angulos_patas = (0, 144, -144)
    else:
        angulos_patas = (0, 72, -72, 144, -144)      # todas apoyadas
    for ang in angulos_patas:
        pata(ang)


construir()

# ---------------------------------------------------------------- escritura
salida = ['# Rocky; generado por crear_rocky.py\n', 'mtllib rocky.mtl\n']
for k, ps in PIVOTES.items():
    salida.append('# pivote %s %s\n' % (k, ' '.join('%s=(%.3f,%.3f,%.3f)' % (n, *p) for n, p in ps.items())))
salida += ['v %.6f %.6f %.6f\n' % v for v in V]
total = 0
for m in COLORES:
    if not F[m]:
        continue
    salida.append(f'usemtl {m}\n')
    salida += [f'f {a} {b} {c}\n' for a, b, c in F[m]]
    total += len(F[m])
(SALIDA / f'{NOMBRE}.obj').write_text(''.join(salida))
(SALIDA / 'rocky.mtl').write_text(''.join(
    f'newmtl {m}\nKd {r} {g} {b}\n\n' for m, (r, g, b) in COLORES.items()))
print(f'Rocky: {len(V)} vértices, {total} triángulos -> {SALIDA}')
