# Aventura espacial

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
| Enter estando en órbita | Entrar al nivel submarino |
| Backspace | Abandonar órbita o volver del nivel al espacio |
| T | Alternar la apariencia nave/carro de Buzz o Rayo McQueen en vuelo libre |
| R | Reiniciar posición o recorrido submarino |
| F11 | Alternar pantalla completa |
| P | Guardar `captura.png` |

Completa el recorrido submarino esquivando obstáculos. Buzz cambia automáticamente a submarino en ese nivel y recupera su forma anterior al salir. La forma carro es una transformación visual con los controles del vuelo libre; no incorpora un circuito terrestre ni física de conducción.

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
