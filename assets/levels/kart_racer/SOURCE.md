# Jardín Rust: procedencia del mapa

Mapa predeterminado de [ZyroBad/kart_racer](https://github.com/ZyroBad/kart_racer), revisión `0f1605af68529dfe85607d64c9c05a848792dd88`.

- `jardin_rust.tiles` reproduce las 41 filas de 65 celdas de `MAP_GARDEN` en `src/main.rs`.
- `jardin_rust.obj` convierte esas celdas a geometría 3D con seis unidades por celda. Conserva trazado, bordes, meta, zonas marcadas, setos, fuente y ubicación de la decoración. La decoración se representa con geometría propia en lugar de los sprites del raycaster.
- Los colores de pista, bordes, meta, marcas y agua se basan en `src/raycaster.rs`.
- Posición inicial original: `(15, 34)`, mirando hacia `+X`.
- La colisión conserva las celdas sólidas de `src/player.rs`: `#`, `H`, `S`, `W`, `D`.

La geometría del mapa y las celdas proceden del proyecto de referencia. Su flujo de menús, vehículos, audio y código de juego no se incorporaron. Las mecánicas de carrera se reimplementaron para este proyecto en `src/race_track/`: conducción arcade, derrape y turbo, checkpoints, tres vueltas, colisiones, minimapa, HUD, pausa y resultados. En pista se renderiza la nave espacial elegida por el jugador usando el renderizador existente.

Al salir con Backspace, el juego devuelve la nave al espacio libre alrededor del planeta de carreras. El mismo comportamiento de retorno al espacio se aplica al abandonar los minijuegos acuático y Guardián del Núcleo.

`assets/planets/planeta_carreras_detallado.obj` es un planeta nuevo modelado en Blender para esta integración, con terreno orgánico, cinturón de pista a cuadros, guardarraíles, luces de impulso y balizas polares. Su fuente editable es `assets/planets/planeta_carreras_detallado.blend` y puede regenerarse con `scripts/create_racing_planet_blender.py`.

Para reconstruir los OBJ, coloca la revisión indicada en `target/kart_racer_source` y ejecuta `python tools/import_kart_map.py` desde la raíz del proyecto. El juego usa los OBJ y las celdas ya incluidos; no necesita descargar el repositorio de referencia.
