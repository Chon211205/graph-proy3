# Aventura espacial

Proyecto de exploración espacial y minijuegos, desarrollado en Rust. El jugador elige una nave, recorre galaxias en vuelo libre, entra en órbita para descubrir misiones y puede volver al espacio al salir de cada nivel.

# Integrantes
- Luis Alfredo Lee Hsu - 241203
- David Sebastián Lemus Nitsch - 241155
- Luis Alejandro Hernández Márquez - 241424

## Requisitos y ejecución

- Rust y Cargo con soporte para la edición 2024.
- Windows es la plataforma recomendada. La música de los minijuegos usa Windows MCI; en otros sistemas esa capa queda en silencio.
- Las dependencias directas son minifb para la ventana y png para cargar imágenes. La geometría 3D, el framebuffer y el rasterizador son propios del proyecto.
- Ejecuta desde la raíz del repositorio, porque el juego carga los recursos desde rutas relativas.

    cargo run --locked

Cargo descarga las dependencias en la primera compilación. El perfil de desarrollo usa optimización ligera para mantener ágil el renderizado por CPU.

## Recorrido

### Menú y hangar

En el menú principal puedes empezar, consultar las instrucciones o salir. El hangar ofrece seis naves:

- Arwing
- Kirby UFO
- ASTRA
- Swordfish II
- Buzz Lightyear
- Rayo McQueen

Usa A/D o las flechas izquierda/derecha para elegir una nave, V para alternar sus variantes disponibles y Enter para iniciar. Las variantes espaciales, de carro y submarinas dependen de cada nave.

### Vuelo libre y galaxias

Explora tres regiones conectadas por agujeros de gusano:

- Sistema solar de origen: el Sol y los planetas principales.
- Vesper: una región con agujero negro, planetas y misiones.
- Nova: la galaxia final, con el Castillo de Lava y su santuario.

Los portales permiten viajar entre regiones y usar la transición de hipervelocidad. Las galaxias también incluyen nebulosas, estrellas, cinturones y asteroides, y encuentros de naves con intercambios de disparos. El vuelo incluye impulso de velocidad de luz, armas, escudo, colisiones y entrada en órbita.

### Planetas y misiones

Las misiones se inician al acercarse y entrar en órbita. La nave elegida y el progreso de la aventura se conservan al regresar al vuelo libre.

| Región / planeta | Modo |
| --- | --- |
| Jardines de Óxido | Gran Premio Orbital |
| Nova Rosa | Guardián del Núcleo |
| Abismo Azul | Ruinas Acuáticas |
| Reloj de Arena | Rescate a Contratiempo |
| Casa Torre | Torre: Última Guardia |
| Castillo de Lava | Combate contra el guardián |
| Santuario de Nova | Nivel final y sello |

La guía Nexo presenta la señal y el objetivo del planeta. Las misiones forman una aventura conectada por la búsqueda de siete esferas; completar objetivos desbloquea nuevas etapas. Backspace permite salir de un minijuego al vuelo espacial. Según el modo, Enter confirma o reinicia desde la pantalla de resultados.

### Modos de juego

- Gran Premio Orbital: carrera 3D con la variante de carro de la nave, circuito cerrado, checkpoints, turbo, derrape, rivales y cuatro vueltas.
- Ruinas Acuáticas: recorrido submarino con formaciones enemigas, obstáculos, combate contra el jefe tiburón y santuario de recompensa.
- Guardián del Núcleo: combate espacial contra un jefe en el planeta rosado, con disparos, objetivos, obstáculos y fases.
- Torre: Última Guardia: galería de tiro con cámara fija, mira, grupos de enemigos, jefes, munición limitada y resultados.
- Rescate a Contratiempo: vuelo de rescate con cápsulas, inversión de gravedad, enemigos, recursos de tiempo y límite de misión.
- Castillo de Lava y Santuario de Nova: combate contra el guardián y etapa interior final.

Los modos tienen pausa, interfaz y resultados propios. Las pistas musicales de los minijuegos se repiten; desde la pausa puedes ajustar el volumen con +/− o silenciar con M.

## Controles

### Vuelo libre

| Teclas | Acción |
| --- | --- |
| W / S o ↑ / ↓ | Subir / bajar |
| A / D o ← / → | Girar |
| Q / E | Alabeo |
| Espacio / Ctrl | Acelerar / frenar |
| Shift | Impulso de velocidad de luz |
| Enter | Disparar; mantener para cargar cuando esté disponible |
| 1 / 2 / 3 | Atajos para acercarse a zonas de misión |
| Enter cerca de un planeta | Entrar en la misión disponible |
| Backspace | Salir de la zona orbital al espacio |
| T | Alternar nave/carro para Buzz o Rayo McQueen cuando está disponible |
| R | Reiniciar posición o segmento, según el modo |

### Carrera

| Teclas | Acción |
| --- | --- |
| W / ↑ | Acelerar |
| S / ↓ | Frenar o reversa |
| A/D o ←/→ | Girar |
| Espacio | Derrapar |
| Shift | Usar turbo |
| R | Recuperar el último checkpoint |
| P / Esc | Pausar |
| Enter | Reintentar desde resultados |
| Backspace | Salir al vuelo libre |

### Ruinas Acuáticas

W/S o ↑/↓ controlan la altura; A/D y ←/→ ajustan la posición lateral. Espacio dispara, Enter dispara o carga según el arma y Backspace vuelve al espacio.

### Guardián del Núcleo

W/A/S/D o las flechas mueven la nave; Espacio o Enter dispara. P/Esc pausa y Backspace sale al espacio.

### Torre: Última Guardia

El mouse o las flechas mueven la mira; clic izquierdo o Espacio dispara. R recarga durante los jefes y P/Esc pausa. En resultados, Enter reintenta y Backspace vuelve al espacio.

### Rescate a Contratiempo

W/S o ↑/↓ controlan la altura y A/D giran. Espacio acelera, Ctrl frena, Shift activa el impulso, Enter dispara y E rescata una cápsula cercana. R reinicia; P/Esc pausa y Backspace vuelve al espacio. P/Esc pausa y Backspace vuelve al vuelo libre.

### Opciones globales

| Tecla | Acción |
| --- | --- |
| Esc | Pausar o volver, según la pantalla |
| F1 | Abrir instrucciones |
| F2 | Alternar resolución dinámica |
| F3 | Mostrar/ocultar métricas compactas de rendimiento |
| F4 | Alternar iluminación avanzada |
| F11 | Alternar pantalla completa cuando el modo lo admite |
| P | Guardar una captura como captura.png |
| C | Activar o salir del cooperativo local cuando está disponible |

En pausa, usa +/− para ajustar la música y M para silenciarla. Los controles específicos del modo también se muestran en el título de la ventana y en su interfaz.

## Estructura del proyecto

- src/main.rs: inicio, estados globales, vuelo, órbitas y bucle principal.
- src/framebuffer.rs y el rasterizador en src/main.rs: dibujo 3D por software.
- src/obj_loader.rs y src/ship.rs: carga OBJ/MTL, materiales y configuración de naves.
- src/galaxy.rs, src/space_decor.rs y src/space_battle.rs: portales, decoración espacial y encuentros entre naves.
- src/story_guide.rs: señales de misión y guía Nexo.
- src/race_track/: circuito, conducción, progreso, rivales, escena e interfaz.
- src/core_guardian.rs y src/core_guardian/: minijuego del Guardián del Núcleo.
- src/tower_guard/: rondas, enemigos, jefes, impactos, escena, sprites, interfaz y audio.
- src/hourglass_rescue.rs y src/hourglass_rescue/: vuelo, cápsulas, escena, interfaz, resultados y audio.
- src/lava_castle.rs y src/lava_castle_interior.rs: encuentros y nivel final.
- assets/: modelos, materiales, imágenes, sonidos, música y recursos de interfaz.
- scripts/: creación, preparación, optimización y vista previa de recursos.

## Recursos 3D y Blender

Los OBJ/MTL son los formatos de carga del juego. Varios recursos también incluyen archivos BLEND editables y scripts Blender para recrearlos o exportarlos. Blender es opcional para ejecutar el juego; solo se necesita para editar o generar esos recursos.

Ejemplos de scripts disponibles:

    blender --background --python scripts/create_sun_blender.py
    blender --background --python scripts/create_racing_planet_blender.py
    blender --background --python scripts/create_tower_house_planet_blender.py
    blender --background --python scripts/create_lava_guardian_blender.py

Las imágenes originales y las referencias de personajes se mantienen en sus carpetas de recursos; los scripts de preparación generan sprites listos para el juego cuando corresponde. Consulta los README específicos dentro de assets para las instrucciones detalladas de cada conjunto.

## Notas de implementación

- La escena principal y los minijuegos usan el renderizador y el framebuffer del proyecto; Blender se utiliza para preparar recursos, no como motor durante la partida.
- Los modelos y texturas se cargan desde disco al iniciar y se reutilizan durante el juego.
- El cooperativo local está disponible en los modos que lo admiten; los menús indican la opción y los controles correspondientes.
- No se requiere una instalación separada de Raylib. Las dependencias Rust del proyecto están declaradas en Cargo.toml.
