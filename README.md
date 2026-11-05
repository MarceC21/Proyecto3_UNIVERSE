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
| M | Mapa interestelar; M, X o Escape para cerrarlo |

Los atajos trasladan la nave en modo seguimiento y la cámara en modo libre.
Las colisiones de la nave con planetas y asteroides usan subpasos, rebote,
daño y reaparición. El acoplamiento y las colisiones de la estación están pendientes.
Los planetas actualmente rotan sobre su eje; sus posiciones orbitales son fijas.

## Marte: terminal de Turing

Acércate a Marte a una altura de 6.5 unidades o menos y una velocidad de 6 o
menos. El aterrizaje se activa automáticamente al llegar despacio. Para probarlo
rápido, pulsa **5** en modo nave; en cámara libre (**V**) no se activa la misión.
Se anima el aterrizaje de la nave durante cinco segundos. Después se muestran
las instrucciones: componentes de la máquina, lectura de las reglas y objetivo
del minijuego. Pulsa **Comenzar** o **Enter** para abrir la terminal.
El paisaje de Marte está generado con
geometría, ruido y shaders del rasterizador en CPU.

- Selecciona una de las cuatro reglas visibles.
- Selecciona el siguiente estado, el símbolo que escribirás y la dirección.
- Pulsa **Ejecutar** o **Enter**. No hay que escribir respuestas.
- También puedes usar **Tab** o **Arriba/Abajo** para elegir un campo y
  **Izquierda/Derecha** para cambiar su opción; **1–4** seleccionan una regla.
- Una transición inválida conserva la cinta y el estado. Una válida anima la
  escritura y el cabezal y carga una etapa del reactor.
- Al alcanzar `qf` en cuatro transiciones se desbloquean el motor, el mapa y la
  ruta a Gargantua. Marte queda completado y la terminal no vuelve a abrirse
  durante esa partida. Al cerrar y ejecutar de nuevo, hay que completarlo otra vez.
- **Salir**, **X** o **Escape** devuelven la nave al espacio desde el aterrizaje,
  las instrucciones o la terminal. Si una transición ya era válida, se completa
  al salir. Si no has terminado, puedes regresar y continuar durante la sesión.
  El avance, los desbloqueos y la ruta se conservan solo en memoria mientras
  está abierto el juego. Al reiniciar se vuelve a empezar desde cero.

La simulación de vuelo se pausa mientras estás en la terminal. En el espacio,
Escape sigue cerrando el programa.

## Mapa y preparación del salto

Pulsa **M** para consultar el mapa. Al principio solo tu sistema tiene nombre;
los otros tres destinos son siluetas negras sin identidad. La carta indica que
la terminal de Marte está pendiente y no permite trazar rutas bloqueadas.

Completar Marte revela **Gargantua** y la selecciona como ruta inicial. Puedes
seleccionar los destinos disponibles con clic o **1–4**. Tau Ceti y 40 Eridani A
siguen ocultos; sus futuras misiones desbloquearán las siguientes rutas en orden.
El mapa pausa el vuelo y se cierra con **M**, **X** o **Escape**.

Con el motor desbloqueado y una ruta trazada, acercarse a **6 unidades del límite
radial** activa automáticamente la sincronización del salto. Solo lo activa la
nave viva, nunca la cámara libre. Sin el motor, el límite conserva su colisión.

Por ahora la siguiente galaxia no está implementada: al completar la carga se
muestra **Enlace listo. Escena de destino pendiente.** La solicitud de salto
conserva origen y destino para conectar esa escena después; no se simula una
llegada a una galaxia inexistente. Cierra el aviso para volver a una posición
segura dentro del sistema. El motor permanece desbloqueado y no hay consumo
ni inventario de combustible todavía.

El juego no lee ni escribe archivos de partida. Cada ejecución comienza con
Marte pendiente, motor y mapa bloqueados y sin ruta. Si existe un archivo antiguo
`partidas/progreso.txt`, se ignora; no hace falta borrarlo para empezar de nuevo.

## Organización

- `src/motor/`: cámara genérica, cielo, matemáticas, geometría, texturas y renderizado.
- `src/juego/navegacion/nave.rs`: carga del modelo, vuelo, modo local, colisiones y emisión de mallas.
- `src/juego/navegacion/seguimiento.rs`: cámara de seguimiento de la nave.
- `src/galaxias/sistema_solar/`: escena, estación y generadores de planetas.
- `assets/compartidos/nave/`: OBJ y MTL copiados del proyecto previo `Lab4/nave_raster`.
- `assets/galaxias/sistema_solar/estacion/`: modelo, materiales y generador de la estación.
- `src/bin/exportar.rs`: vistas sin ventana, incluidas nave, estación y planetas exteriores.
- `src/minijuegos/puzzle/turing.rs`: reglas, validación, cinta y ejecución de la máquina.
- `src/juego/estados/marte.rs`: llegada, descenso, terminal y salida de Marte.
- `src/juego/estados/progreso.rs`: desbloqueos de la partida actual, solo en memoria.
- `src/juego/navegacion/mapa.rs`: mapa y selección de ruta.
- `src/juego/navegacion/salto.rs`: activación en el borde y solicitud para la futura escena.
- `src/galaxias/sistema_solar/marte/`: paisaje procedural y presentación de la terminal.

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
`marte_superficie.ppm` permite revisar el paisaje de la misión sin abrir ventana.
`marte_aproximacion.ppm`, `marte_descenso.ppm` y `marte_contacto.ppm` muestran
las etapas del aterrizaje con la nave.
