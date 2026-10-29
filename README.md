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

El carro `assets/vehicles/mcqueen_carro.obj` tiene carrocería roja, rayos y número 95 amarillos, ojos azules y cuatro ruedas negras con llantas rojas. El submarino `assets/vehicles/mcqueen_submarino.obj` mantiene esa paleta e incorpora periscopio, aletas, flotadores, faros y dos hélices con aros protectores. El juego define los colores en su renderizador.

## Modelos de Buzz

Los tres modelos originales inspirados en Buzz usan blanco, verde lima, morado y detalles rojos, cabina azul y botones de colores. Están en `assets/vehicles/buzz_*.obj`; `buzz.mtl` permite ver los materiales también en herramientas externas. El juego define la misma paleta en su renderizador.

Se pueden reconstruir, sin paquetes adicionales, con:

```powershell
python scripts/generate_buzz.py
```

Pruebas de entrada y de carga de los modelos:

```powershell
cargo test --locked
```
