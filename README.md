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

Al completar los dos segundos de sincronización se carga **Gargantua** con su
propio cielo, límites y punto de aparición. El mapa muestra la ubicación actual.
Desde Gargantua puedes seleccionar el sistema solar y regresar llegando al borde.
**X** o **Escape** permiten cancelar la sincronización y volver a una posición
segura del sistema actual. Viajar no completa Miller ni desbloquea Tau Ceti.
El motor permanece desbloqueado durante la sesión; no hay inventario ni consumo
de combustible todavía.

El juego no lee ni escribe archivos de partida. Cada ejecución comienza con
Marte pendiente, motor y mapa bloqueados y sin ruta. Si existe un archivo antiguo
`partidas/progreso.txt`, se ignora; no hace falta borrarlo para empezar de nuevo.

## Gargantua: navegación

Para inspeccionar directamente el nuevo sistema:

```bash
cargo run --release -- --galaxia gargantua
```

Ese acceso inicia una sesión de inspección con el motor ya habilitado. El inicio
normal con `cargo run --release` conserva la misión de Marte y el mapa bloqueado.
También se puede combinar `--galaxia solar|gargantua` con una ruta OBJ de nave.

Gargantua tiene un **agujero negro** central con horizonte oscuro, disco de
acreción animado y un arco luminoso, todo rasterizado en CPU. Sus seis planetas son Miller (océano, el más cercano), Mann (hielo agrietado), Edmunds
(roca), Thalos (bandas y dos anillos separados), Nyx (fisuras) y Aurora (mares
y continentes). Las posiciones orbitales permanecen fijas y rotan con `dt`.

No tiene cinturón, asteroides ni estación. El radio del límite es 420 y la altura
75, frente a 350 y 45 del sistema solar. Los planetas tienen radios entre 7 y 11;
las mismas tres mallas de esfera y los mipmaps conservan el presupuesto de detalle.
El skybox usa azul petróleo, turquesa y luz crema, sin la nebulosa violeta de Solar.

Se conservan WASD, ratón, turbo, cámara libre, modo local, colisiones y reaparición.
Los atajos cambian según el sistema:

| Tecla | Destino en Gargantua |
| --- | --- |
| 0 | Vista general |
| 1 | Centro Gargantua |
| 2 | Miller |
| 3 | Mann |
| 4 | Edmunds |
| 5 | Thalos, doble anillo |
| 6 | Nyx |
| 7 | Aurora |
| 8 | Límite |

`R` reaparece en el sistema actual. En el sistema solar los controles anteriores
permanecen iguales. Para generar vistas y medir los tiempos sin ventana:

```bash
cargo run --release --no-default-features --bin exportar_gargantua -- /tmp/gargantua
```

## Agujero negro y atracción

El horizonte tiene **radio 26** (el sol provisional tenía 18), y el disco luminoso
alcanza **radio 68** sin invadir Miller, cuya órbita está a 92 unidades del centro.
Los demás planetas también se han separado hacia fuera. Se conservan las esferas LOD del motor;
el disco y el arco añaden 514 triángulos. La iluminación de los planetas toma
el centro del disco como fuente aproximada.

La nave siente una atracción radial dentro de **150 unidades** del centro. La
fuerza crece al acercarse y está limitada a 22 unidades de aceleración para
mantener estable la integración. La física es propia: no se usan motores ni
funciones de física de raylib. Los controles siguen funcionando; orienta la
nave hacia fuera y mantén **W + Shift** para compensar la caída. Si activaste el
modo local hacia el agujero, **F** libera la orientación para poder girar.
El modo local no fija la distancia al agujero ni cancela su gravedad.

Al cruzar el horizonte, la nave se pierde y reaparece tras 2.5 segundos en el
punto seguro de Gargantua, conservando el progreso de la partida. La cámara
libre no recibe gravedad. El sistema solar mantiene su vuelo anterior.

Es una aproximación de juego, con disco y arco artísticos; no se integra la
relatividad ni la trayectoria de la luz. La gravedad de superficie propia de
Miller queda para después; su misión se describe a continuación.

Para verlo, inicia `cargo run --release -- --galaxia gargantua` y pulsa **1**.
El exportador incluye `agujero_cerca.ppm` y `agujero_lateral.ppm` para inspeccionar
el encuadre sin ventana.

## Misión de Miller

En Gargantua, acércate a Miller a menos de 6.5 unidades de altura y baja la
velocidad a 6 o menos para aterrizar. La cámara libre no activa la misión.
La nave realiza una aproximación y un descenso de cinco segundos. Después se
presentan las instrucciones en azul: pulsa **Comenzar / Enter** para iniciar
el reloj y el avance de la ola. La nave de superficie tiene escala **×5**;
su tamaño durante el vuelo espacial se conserva.
Para probar directamente la misión dentro del mismo juego:

```bash
cargo run --release -- --miller
```

- **WASD**: caminar sobre el agua; **ratón / flechas**: mirar; **Tab**: cursor.
- **E**: inspeccionar una caja a menos de 3.5 unidades. Hay tres; el módulo
  cambia aleatoriamente al comenzar cada intento. Las revisadas quedan verdes.
- **Clic o 1–3**: responder las cuatro preguntas y seleccionar una estrategia.
  Una respuesta incorrecta permite volver a intentar; el reloj sigue avanzando.
  El cuestionario usa un panel morado. Al terminar, aparece una pantalla naranja
  que confirma la recuperación de datos y permite elegir entre las tres formas
  de regresar a la nave.
- **TARS**: transporte asistido hacia la nave, evitando las cajas. No requiere
  todavía un modelo de TARS. **Correr** y **equipo adicional** mantienen WASD;
  la velocidad depende de la distancia a la nave y de la estrategia elegida.
- **X / Esc**: abandonar y volver al vuelo. En la pantalla final, **R** inicia
  un intento limpio con reloj, cajas, cuestionario, controles y ola restaurados.

El cronómetro usa `Instant` y mide tiempo real, incluso durante las preguntas.
La ola se acerca desde el inicio; perder depende de su alcance espacial. Los
valores internos de diseño son 90 s para un regreso exitoso y 110 s para que
el frente alcance la zona de la nave. Se ajustan en `Configuracion::default()`
de `src/juego/estados/miller.rs`; no se muestran como cuenta regresiva.
Las estrategias usan referencias de 8, 18 y 35 s para calcular velocidades,
sin sumar esos segundos al reloj ni mostrarlos al jugador.

Solo al finalizar se revela el tiempo exterior: **1 hora en Miller = 7 años**,
equivalente a `segundos_locales * 7 * 365 / 3600` días. Hay finales de éxito,
regreso tardío y derrota por la ola. Solo el éxito completa Gargantua y revela
Tau Ceti en el mapa; ese sistema todavía no permite viajar. La misión exitosa
no se reabre al acercarse a Miller durante la misma partida. Todo el progreso
se reinicia al cerrar el juego, como en Marte.

Se conserva el agua procedural, la pared de agua y las cajas de la escena
entregada. El jugador reutiliza `Camara`, su suavizado y sus colisiones, con
altura fija y subpasos; la nave de superficie es el OBJ compartido. No se
añade otro bucle de juego ni renderizado 3D de raylib.

```bash
cargo run --release --no-default-features --bin exportar_miller -- /tmp/miller
cargo run --release --bin ver_miller
```

El exportador muestra el avance de la ola a 0, 60 y 100 s sin ventana.
También guarda `miller_aproximacion.ppm`, `miller_descenso.ppm` y
`miller_contacto.ppm` para revisar la animación de llegada.
`ver_miller` conserva la vista previa visual entregada (flechas para mirar),
mientras `--miller` abre el minijuego interactivo integrado.

## Organización

- `src/motor/`: cámara genérica, cielo, matemáticas, geometría, texturas y renderizado.
- `src/juego/navegacion/nave.rs`: carga del modelo, vuelo, modo local, colisiones y emisión de mallas.
- `src/juego/navegacion/seguimiento.rs`: cámara de seguimiento de la nave.
- `src/galaxias/sistema_solar/`: escena, estación y generadores de planetas.
- `src/galaxias/gargantua/`: configuración del segundo sistema, seis texturas y agujero negro.
- `src/galaxias/gargantua/miller/`: paisaje y controlador de la misión de superficie.
- `src/juego/estados/miller.rs`: cajas, preguntas, estrategias, ola, reloj y resultados.
- `src/motor/fisica/gravedad.rs`: campo de atracción independiente de raylib.
- `src/motor/cielo.rs`: generador de skybox para cada sistema.
- `assets/compartidos/nave/`: OBJ y MTL copiados del proyecto previo `Lab4/nave_raster`.
- `assets/galaxias/sistema_solar/estacion/`: modelo, materiales y generador de la estación.
- `src/bin/exportar.rs`: vistas del sistema solar sin ventana.
- `src/bin/exportar_gargantua.rs`: vistas y tiempos del nuevo sistema sin ventana.
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
