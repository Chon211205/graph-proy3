# Aventura espacial

## Planeta de carreras: Gran Premio Orbital

En el sistema solar inicial encontrarás el planeta de carreras con su cinturón a cuadros. Acércate hasta entrar en órbita y pulsa **Enter** para correr **Gran Premio Orbital**. La tecla **3** te lleva junto al planeta para entrar más rápido. La carrera carga el modelo OBJ del carro asociado a la nave seleccionada, cuenta regresiva, circuito cerrado, tres rivales, checkpoints ordenados y tres vueltas.

El escenario tiene carretera oscura, pianos rojos y blancos, bandas turbo, barreras, vegetación alienígena, árboles grandes, helechos, arcos musgosos, colinas y un planeta anillado al fondo. El circuito y su decoración se generan una vez desde los módulos de `src/race_track/`; se usa el renderizador de software del proyecto.

Hay variantes carro para ASTRA, Swordfish II, Buzz Lightyear y Rayo McQueen. Arwing y Kirby todavía no tienen modelo carro, así que el juego no los sustituye por un modelo genérico. Los archivos que faltan son `assets/vehicles/arwing/arwing2_carro.obj` y `assets/vehicles/kirby/kirby_carro.obj`.

| Tecla | Acción en la carrera |
| --- | --- |
| W / arriba | Acelerar |
| S / abajo | Frenar o ir en reversa |
| A/D o flechas izquierda/derecha | Girar |
| Espacio | Freno de mano y derrape |
| Shift | Consumir reserva de turbo |
| R | Volver al último checkpoint |
| P / Esc | Pausar o continuar |
| Enter | Reintentar al terminar |
| Backspace | Salir del planeta y volver al espacio libre |

Shift consume la reserva y se recarga al soltarlo; derrapar de forma controlada y cruzar una banda turbo da una recarga breve. Los resultados indican posición, tiempo total y mejor vuelta. Backspace regresa al espacio libre. La carrera no incluye el menú principal ni la música del proyecto de referencia; la lógica, IA y geometría están adaptadas a la estructura y al renderizador de este proyecto.

Juego 3D en Rust con renderizado por software. Ejecuta los comandos desde la raíz del repositorio para encontrar `assets/`.

```powershell
cargo run --locked
```

Se necesita Rust con soporte para edición 2024 y las herramientas nativas de compilación de las dependencias. La primera ejecución descarga y compila las dependencias de Cargo.

## Menús

- Menú principal: **Jugar**, **Instrucciones**, **Salir**.
- Flechas arriba/abajo o W/S y Enter para elegir una opción.
- Hangar: A/D o flechas izquierda/derecha para elegir entre Arwing, Kirby, ASTRA, Swordfish II y Buzz Lightyear. Enter inicia la partida; Esc vuelve al menú.
- En el hangar, V alterna las formas disponibles: todas las naves tienen modo espacial y submarino; ASTRA, Swordfish II y Buzz también tienen modo carro.
- Esc pausa y permite continuar, cambiar de nave, consultar las instrucciones o salir. Cambiar de nave comienza una partida nueva.
- F1 muestra los controles durante la partida; Enter/Esc vuelve. La simulación se detiene en pausa y en la ayuda.

## Controles

| Tecla | Acción |
| --- | --- |
| W/S o arriba/abajo | Subir/bajar |
| A/D | Girar |
| Q/E | Alabeo |
| Espacio / Ctrl | Acelerar / frenar |
| Shift | Velocidad de luz en vuelo libre |
| Enter | Disparar; mantener para cargar |
| 1 | Transportarse cerca del planeta acuático |
| 3 | Transportarse cerca del planeta de carreras |
| Enter estando en órbita | Entrar al nivel submarino |
| Backspace | Abandonar órbita o volver del nivel al espacio |
| T | Alternar la apariencia nave/carro de Buzz o Rayo McQueen en vuelo libre |
| R | Reiniciar posición o recorrido submarino |
| F11 | Alternar pantalla completa |
| F3 | Mostrar/ocultar métricas de rendimiento |
| P | Guardar `captura.png` |

Completa el recorrido submarino esquivando obstáculos. Buzz cambia automáticamente a submarino en ese nivel y recupera su forma anterior al salir. La forma carro es una transformación visual con los controles del vuelo libre; no incorpora un circuito terrestre ni física de conducción.

El recorrido submarino sigue la estructura **Ruinas Acuáticas**: caverna de
entrada, corredor de ruinas, campo minado, túnel de alta velocidad con anillos
de impulso, arena abierta y combate final contra un tiburón mecánico. El avance
se detiene en la última arena hasta destruir al jefe con **Enter**; los disparos
cargados causan más daño.

Después del jefe se abre una séptima sección: el santuario final. Una esfera
del dragón naranja de cuatro estrellas flota sobre un pedestal energético. Hay
que acercarse para recogerla; hasta entonces, la salida y el final del nivel
permanecen bloqueados.

El santuario es una cámara cerrada con suelo, techo, paredes y un único acceso.
La compuerta permanece bloqueada durante el combate y la explosión del tiburón;
después se eleva durante 1.8 segundos y permite entrar al cuarto de recompensa.
Al recoger la esfera, el submarino queda inmóvil y aparece la pantalla de
**MISIÓN CUMPLIDA** durante 3.4 segundos. Al terminar, el jugador regresa
automáticamente al vuelo libre junto al planeta acuático.

Los enemigos avanzan en formaciones de cuña, diamante, línea y muro de minas.
Tiburones y medusas disparan por turnos contra el submarino; el jefe lanza
ráfagas triples. El casco soporta seis impactos y, al agotarse, devuelve al
jugador al tramo anterior con una breve protección de reaparición.

Todos los modos de juego comparten un HUD de combate: escudo, objetivo
contextual, puntuación, altitud, impulso, carga del bláster, retícula, radar y
barra de vida para los jefes. Vuelo libre, Ruinas Acuáticas, Jardín Rust y el
Guardián del Núcleo también reutilizan la apariencia y los colores del bláster
de la nave inicial seleccionada. En el combate del Guardián se usa además la
misma mecánica: una pulsación corta dispara el tiro doble normal y mantener para
soltar después genera un proyectil cargado más grande, luminoso y potente.

## Rayo McQueen

En el hangar, selecciona Rayo McQueen y pulsa **V** para alternar nave/carro/submarino. **Enter** inicia con la forma de Rayo que estés viendo; si eliges submarino, empiezas directamente en el nivel acuático. Durante el vuelo libre, **T** alterna nave/carro conservando posición y velocidad. El carro usa los controles de vuelo existentes, sin física terrestre.

Desde el espacio, **1** te acerca al planeta acuático. Ya en órbita, **Enter** inicia el nivel y transforma a Rayo en submarino. **Backspace** vuelve al espacio y recupera la forma anterior; también se recupera al completar el recorrido. Si empezaste directamente en submarino, regresarás en forma de nave.

El carro `assets/vehicles/mcqueen/mcqueen_carro.obj` tiene carrocería roja, rayos y número 95 amarillos, ojos azules y cuatro ruedas negras con llantas rojas. El submarino `assets/vehicles/mcqueen/mcqueen_submarino.obj` mantiene esa paleta e incorpora periscopio, aletas, flotadores, faros y dos hélices con aros protectores. El juego define los colores en su renderizador.

## Modelos de Buzz

Los tres modelos originales inspirados en Buzz usan blanco, verde lima, morado y detalles rojos, cabina azul y botones de colores. Están en `assets/vehicles/buzz/buzz_*.obj`; sus materiales permiten ver la misma paleta en herramientas externas y en el juego.

Se pueden reconstruir, sin paquetes adicionales, con:

```powershell
python scripts/generate_buzz.py
```

Pruebas de entrada y de carga de los modelos:

```powershell
cargo test --locked
```

## Modelado con Blender mediante MCP

El sol detallado se conserva como `assets/planets/sol_detallado.blend` y se
exporta a OBJ para el juego. Para reconstruirlo con Blender 4.x:

```powershell
blender --background --python scripts/create_sun_blender.py
```

El planeta de carreras también conserva su archivo Blender editable y se puede
regenerar con:

```powershell
blender --background --python scripts/create_racing_planet_blender.py
```

En la galaxia Vesper, la antigua caracola espiral fue reemplazada por el
planeta-casa de torre. Su fuente editable se guarda como
`assets/planets/caracola.blend`; el nombre se conserva para no romper la carga
del juego. Para regenerar el BLEND, OBJ y MTL:

```powershell
blender --background --python scripts/create_tower_house_planet_blender.py
```

Los enemigos de **Ruinas Acuáticas** también tienen una fuente Blender editable:
la mina globo blindada, el tiburón dron (usado además como jefe) y la medusa
robótica. Para regenerar sus OBJ y MTL optimizados:

```powershell
blender --background --python scripts/create_aquatic_enemies_blender.py
```

`blender_mcp/` permite que un cliente MCP controle Blender para crear modelos a
partir de cubos, esferas, cilindros, conos y toros, inspeccionar la escena y
exportar OBJ/MTL directamente a `assets/`. La comunicacion es local
(`127.0.0.1:9876`) y Blender solo acepta el conjunto de comandos definido por el
complemento; no expone ejecucion arbitraria de Python.

1. Instala Blender 4.x y, en **Edit > Preferences > Add-ons > Install from
   Disk**, selecciona `blender_mcp/addon.py`. Activa **Local MCP Bridge**.
2. Crea un entorno virtual e instala el SDK oficial:

   ```powershell
   py -3.11 -m venv .venv
   .\.venv\Scripts\python -m pip install -r blender_mcp\requirements.txt
   ```

3. Copia `blender_mcp/mcp-config.example.json` a la configuracion de tu cliente
   MCP, sustituye las rutas de ejemplo por rutas absolutas y usa el Python del
   entorno virtual como `command`.
4. Reinicia el cliente MCP y comprueba la conexion con `blender_status`.

Las herramientas disponibles permiten limpiar e inspeccionar la escena, crear
una primitiva, transformar un objeto, construir hasta 500 piezas de una vez,
guardar el `.blend` y exportar `.obj`. Por seguridad, las dos ultimas operaciones
solo escriben dentro de `assets/`.

Si se define `BLENDER_MCP_TOKEN`, debe tener el mismo valor tanto en el entorno
que inicia Blender como en la entrada `env` del servidor MCP.

Pruebas del puente (no requieren Blender):

```powershell
python -m unittest discover -s tests -v
```

### Interceptor azul de referencia

El modelo `assets/vehicles/arwing/arwing2.obj` incluye materiales MTL para el
juego y visores OBJ. Para obtener los materiales PBR completos en Blender
(metal pintado, laca, cristal y emisión), ejecuta desde la raíz:

```powershell
blender --background --python scripts/setup_interceptor_blender.py
```

El script importa el OBJ, crea shaders Principled BSDF, aplica micro-bisel y
suavizado, configura Eevee y guarda `assets/vehicles/arwing/arwing2.blend`.
