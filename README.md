# Aventura espacial

Juego 3D en Rust con renderizado por software. Ejecuta los comandos desde la raíz del repositorio para encontrar `assets/`.

```powershell
cargo run --locked
```

Se necesita Rust con soporte para edición 2024 y las herramientas nativas de compilación de las dependencias. La primera ejecución descarga y compila las dependencias de Cargo.

## Menús

- Menú principal: **Jugar**, **Instrucciones**, **Salir**.
- Flechas arriba/abajo o W/S y Enter para elegir una opción.
- Hangar: A/D o flechas izquierda/derecha para elegir entre Arwing, Kirby, ASTRA, Cowboy B y Buzz Lightyear. Enter inicia la partida; Esc vuelve al menú.
- Con Buzz seleccionado, V muestra sus versiones nave, carro y submarino en el hangar. La partida comienza en forma de nave.
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
| T | Alternar la apariencia nave/carro de Buzz en vuelo libre |
| R | Reiniciar posición o recorrido submarino |
| F11 | Alternar pantalla completa |
| P | Guardar `captura.png` |

Completa el recorrido submarino esquivando obstáculos. Buzz cambia automáticamente a submarino en ese nivel y recupera su forma anterior al salir. La forma carro es una transformación visual con los controles del vuelo libre; no incorpora un circuito terrestre ni física de conducción.

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
