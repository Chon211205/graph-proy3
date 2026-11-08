# Torre: Última Guardia

Este minijuego se abre al orbitar la Casa Torre, el tercer cuerpo de la galaxia Vesper, y pulsar Enter. Al volver, la nave conserva su selección y su órbita.

Los PNG de `source/` son las referencias originales y permanecen intactos. `enemy_manifest.tsv` define la identidad, salud y zonas de impacto de los seis enemigos y los dos jefes. Los sprites transparentes se regeneran con; el proceso también elimina islas blancas grandes encerradas entre extremidades y conserva los detalles blancos pequeños de caras y dientes:

```powershell
python scripts/prepare_tower_guard_sprites.py
```

Los sonidos cortos de disparo, impacto, daño y aviso se pueden regenerar sin bibliotecas externas con `python scripts/prepare_tower_guard_audio.py`.

Los títulos transparentes YOU DIED / YOU WIN se preparan a partir de la referencia original con `python scripts/prepare_tower_guard_result_titles.py`.

El script requiere Pillow. Como cada referencia contiene una pose, el movimiento se simula con avance en profundidad, balanceo, inclinación y animaciones de reacción/caída; no son ciclos dibujados de varias poses.

La arena usa la cámara 2D fija del renderizador del proyecto. Los impactos consultan la máscara alfa del mismo sprite y transformación con que se dibuja. Raylib, que ya forma parte del proyecto, reproduce los sonidos; si no hay dispositivo de audio, el juego sigue funcionando en silencio.

Controles: mouse o flechas para apuntar, clic o espacio para disparar, R para recargar durante los jefes y P/Esc para abrir la pausa común. En resultados, Enter reintenta; Backspace o el botón permite volver a la órbita.
