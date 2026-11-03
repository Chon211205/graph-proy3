# Gran Premio Orbital: procedencia y escena

Mapa predeterminado de [ZyroBad/kart_racer](https://github.com/ZyroBad/kart_racer), revisión `0f1605af68529dfe85607d64c9c05a848792dd88`.

- `jardin_rust.tiles` y `jardin_rust.obj` conservan la pista original de referencia para comparación; el minijuego actual ya no los usa como trazado activo.
- El circuito activo se define por puntos centrales cerrados y se rasteriza en mallas 3D de carretera, pianos, barreras, líneas de meta y bandas turbo en `src/race_track/course.rs`.
- La decoración se genera en `src/race_track/scenery.rs`: terreno verde, árboles de baja geometría, helechos, arcos con musgo, lomas y planeta anillado de fondo.
- La colisión, los checkpoints, la IA y el minimapa consultan la misma línea central del circuito.

La referencia aportó la idea de conducción arcade, superficies, derrape, turbo, vueltas, minimapa y HUD. No se incorporaron su menú, audio, vehículos ni código de juego. La carrera se implementó en `src/race_track/` con conducción, reserva turbo, tres rivales, checkpoints dirigidos, clasificación, colisiones, minimapa y resultados. En pista se renderizan variantes carro OBJ cargadas por el juego: ASTRA, Swordfish II, Buzz y Rayo McQueen.

No existen variantes carro para Arwing ni Kirby en los recursos actuales; el juego muestra qué OBJ falta y no presenta otro modelo como si fuera el suyo.

Al salir con Backspace, el juego devuelve la nave al espacio libre alrededor del planeta de carreras. El mismo comportamiento de retorno al espacio se aplica al abandonar los minijuegos acuático y Guardián del Núcleo.

`assets/planets/planeta_carreras_detallado.obj` es un planeta nuevo modelado en Blender para esta integración, con terreno orgánico, cinturón de pista a cuadros, guardarraíles, luces de impulso y balizas polares. Su fuente editable es `assets/planets/planeta_carreras_detallado.blend` y puede regenerarse con `scripts/create_racing_planet_blender.py`.

La geometría activa se genera determinísticamente al cargar el módulo, sin descargar fuentes ni generar archivos durante los fotogramas. El planeta del sistema conserva su OBJ de Blender; la vegetación y los elementos del circuito se construyen mediante mallas de baja densidad compatibles con el renderizador existente.
