# Proyecto3 UNIVERSE

Sistema solar y nave en Rust con rasterizador por software. Raylib abre la
ventana, recibe entradas y muestra el framebuffer y el HUD. Las mallas,
transformaciones, recorte, shaders y profundidad se procesan en CPU.

## Ejecutar

```bash
cargo run --release
```

La nave incluida se carga automáticamente. También se puede elegir otro OBJ,
con su MTL en la misma carpeta:

```bash
cargo run --release -- /ruta/a/nave.obj
```

El vuelo, la escala, las cinco esferas de colisión y las posiciones de los
motores están configurados para el modelo incluido.

## Controles

| Entrada | Acción |
| --- | --- |
| W / S | Avanzar / retroceder |
| A / D | Desplazamiento lateral |
| Espacio / Ctrl izquierdo | Subir / bajar |
| Ratón o flechas | Orientar la nave |
| Q / E | Alabeo |
| Shift izquierdo | Turbo |
| Botón derecho + ratón | Orbitar la cámara alrededor de la nave |
| F | Activar modo local para rodear un planeta cercano |
| R | Restaurar la nave en el punto inicial |
| V | Alternar cámara libre y seguimiento; la nave queda detenida en la simulación durante la cámara libre |
| Tab | Liberar / capturar cursor |
| C / L / O / B | Alternar culling, LOD, guías orbitales y barrera visible |
| F1 / F2 / F3 | Calidad alta / media / baja |
| 0 | Vista general |
| 1 / 2 / 3 / 4 / 5 | Sol / Mercurio / Venus / Tierra / Marte |
| 6 / 7 | Cinturón / límite |
| 8 / 9 / U / N | Júpiter / Saturno / Urano / Neptuno |
| T | Estación junto a la Tierra |

Los atajos trasladan la nave en modo seguimiento y la cámara en modo libre.
Las colisiones de la nave con planetas y asteroides usan subpasos, rebote,
daño y reaparición. El acoplamiento y las colisiones de la estación están pendientes.
Los planetas actualmente rotan sobre su eje; sus posiciones orbitales son fijas.

## Organización

- `src/motor/`: cámara genérica, cielo, matemáticas, geometría, texturas y renderizado.
- `src/juego/navegacion/nave.rs`: carga del modelo, vuelo, modo local, colisiones y emisión de mallas.
- `src/juego/navegacion/seguimiento.rs`: cámara de seguimiento de la nave.
- `src/galaxias/sistema_solar/`: escena, estación y generadores de planetas.
- `assets/compartidos/nave/`: OBJ y MTL copiados del proyecto previo `Lab4/nave_raster`.
- `assets/galaxias/sistema_solar/estacion/`: modelo, materiales y generador de la estación.
- `src/bin/exportar.rs`: vistas sin ventana, incluidas nave, estación y planetas exteriores.

La integración reutiliza las funciones entregadas en `navecita`, el cargador OBJ
existente y el modelo del proyecto anterior. Se utilizó asistencia de IA para
la integración y las comprobaciones.

## Comprobar sin ventana

```bash
cargo check --all-targets
cargo test --no-default-features
cargo run --release --no-default-features --bin exportar -- /tmp/universe-vistas
```

El exportador crea la carpeta y guarda archivos PPM. Acepta una ruta OBJ
opcional como segundo argumento. El modelo predeterminado se incluye en el
binario para poder ejecutarlo desde otra carpeta.
