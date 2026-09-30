# Proyecto 2: Raytracing

Raytracer en **CPU** escrito a mano en **Rust**. El diorama tiene dos islas flotantes estilo Minecraft unidas por un puente recto de piedra:

- **Overworld**: terreno procedural con Perlin, casa de madera, árbol de roble, estanque con cascada, charco de lava y bloques de oro.
- **Nether**: netherrack con parches de magma, portal de obsidiana, lago y cascada de lava, y glowstone.

## Video

[![Video del diorama](https://img.youtube.com/vi/MtN6htSj5bw/maxresdefault.jpg)](https://www.youtube.com/watch?v=MtN6htSj5bw)

▶️ **Ver en YouTube:** https://www.youtube.com/watch?v=MtN6htSj5bw

## Cómo correrlo

```bash
# Ventana interactiva (rotar / zoom en tiempo real)
cargo run --release

# Imagen fija en alta calidad (4 muestras por píxel)
cargo run --release -- --still diorama.bmp

# Video: 240 frames (vuelta de 360° con acercamiento/alejamiento) en frames/
cargo run --release -- --video 240 1280 720
ffmpeg -y -framerate 30 -i frames/frame_%04d.ppm -c:v libx264 -pix_fmt yuv420p -crf 23 diorama.mp4
```

### Requisitos (Windows)

- Rust (rustup) con el toolchain MSVC
- CMake y Visual Studio Build Tools (C++), para compilar raylib
- LLVM (`libclang.dll`) y la variable `LIBCLANG_PATH`
- ffmpeg, solo para armar el video

### Controles

| Acción | Tecla |
|---|---|
| Rotar alrededor del diorama | Arrastrar con el mouse · A / D · ← / → |
| Inclinar la cámara | W / S · ↑ / ↓ |
| Acercar / alejar (zoom) | Rueda del mouse · Q / E |
| Rotación automática | Espacio |
| Guardar captura BMP | P |

## Restricciones

- **Sin librerías de gráficos ni de matemáticas.** Vectores, rayos, intersecciones, ruido Perlin, texturas, materiales y escritura de imágenes (BMP/PPM) están hechos con la librería estándar.
- **raylib** solo se usa para abrir la ventana, mostrar el framebuffer ya calculado y leer el teclado y el mouse. No calcula nada del render.
- **Sin GPU.** Cada píxel se calcula en la CPU.
- **Texturas generadas por código.** Son pixel-art de 16×16 y no se carga ninguna imagen externa.

## Rúbrica

| Punto | Implementación | Archivo |
|---|---|---|
| Complejidad de la escena | Dos islas (~4800 bloques) con casa, árbol, estanque, cascada, lava, oro, puente, portal, cascada de lava y glowstone | `diorama.rs` |
| Rotación + zoom | Cámara orbital en coordenadas esféricas (yaw, pitch, distancia) con base LookAt, interactiva y en el video | `camera.rs`, `main.rs`, `video.rs` |
| Materiales (18) | Cada bloque tiene su textura y sus propios valores de albedo, specular, shininess, reflectividad, transparencia e IOR | `material.rs`, `texture.rs` |
| Refracción | Agua (IOR 1.333) en el estanque y la cascada, y vidrio (IOR 1.5) en las ventanas: ley de Snell, Fresnel con Schlick, reflexión interna total y absorción Beer-Lambert | `shading.rs`, `math.rs` |
| Reflexión | Oro (reflejo teñido del color del metal) y obsidiana: R = I − 2(I·N)N recursivo | `shading.rs` |
| Normal maps | Puente de stone bricks, cobblestone, piedra, netherrack y agua. Las normales se calculan a partir de un mapa de alturas y se aplican en espacio tangente (T, B, N) | `texture.rs`, `material.rs` |
| Emisivos | Lava, magma, glowstone y portal (este último además es semitransparente). Iluminan su entorno con luces puntuales y sombras | `material.rs`, `light.rs` |
| Skybox | Cube map de 6 caras con un cielo de atardecer procedural (gradiente + nubes de ruido 3D) y disco solar | `skybox.rs` |
| Terreno procedural | Isla de **~21×21** columnas: la altura sale de Perlin 2D (fBm), el borde es irregular por ruido y la parte de abajo tiene forma de estalactita | `perlin.rs`, `diorama.rs` |
| Paralelismo / optimización | Ver la sección siguiente | `render.rs`, `world.rs` |

### Materiales

| Material | Tipo | Albedo | Specular | Reflectividad | Transparencia | Extra |
|---|---|---|---|---|---|---|
| Pasto | Lambert | 1.00 | 0.03 | 0 | 0 | Textura distinta arriba / lados / abajo |
| Tierra | Lambert | 0.95 | 0.02 | 0 | 0 | |
| Piedra | Normal map | 0.90 | 0.10 | 0 | 0 | Normal map |
| Cobblestone | Normal map | 0.95 | 0.15 | 0 | 0 | Normal map (Voronoi) |
| Stone bricks (puente) | Normal map | 0.95 | 0.20 | 0 | 0 | Normal map (ladrillos) |
| Tronco de roble | Lambert | 1.00 | 0.04 | 0 | 0 | Anillos arriba / corteza a los lados |
| Hojas de roble | Lambert | 0.95 | 0.06 | 0 | 0 | |
| Tablas de roble | Blinn-Phong | 0.95 | 0.12 | 0 | 0 | |
| Puerta de roble | Blinn-Phong | 0.95 | 0.15 | 0 | 0 | |
| Vidrio | Dieléctrico | 0.10 | 0.90 | 0.06 | 0.85 | IOR 1.5 |
| Agua | Dieléctrico | 0.25 | 0.90 | 0.05 | 0.80 | IOR 1.333 + normal map |
| Lava | Emisivo | 1.00 | 0 | 0 | 0 | Emisión 3.0 |
| Oro | Metal | 0.85 | 1.00 | 0.45 | 0 | Reflejo teñido |
| Netherrack | Normal map | 0.95 | 0.10 | 0 | 0 | Normal map |
| Obsidiana | Blinn-Phong | 0.85 | 0.80 | 0.05 | 0 | Brillo 96 |
| Portal | Emisivo | 1.00 | 0 | 0 | 0.35 | Emisión 1.8, semitransparente |
| Magma | Emisivo | 1.00 | 0.10 | 0 | 0 | Emisión 1.4 |
| Glowstone | Emisivo | 1.00 | 0 | 0 | 0 | Emisión 2.2 |

## Optimización

1. **Multihilo** (`std::thread::scope`): la imagen se divide en bandas de 4 filas y cada hilo toma la siguiente banda libre de una cola compartida. Así la carga se reparte sola aunque unas zonas sean más caras que otras.
2. **Grilla de voxels + DDA de Amanatides & Woo**: el rayo avanza celda por celda. El costo depende de las celdas que cruza, no del número de cubos de la escena.
3. **Broadphase con AABB**: hay una caja para cada isla y otra para el puente. Si un rayo no toca ninguna, va directo al skybox. Si toca varias, se recorren de la más cercana a la más lejana y se corta en cuanto se encuentra el primer impacto.
4. **Slabs de Kay & Kajiya** con `1/dirección` precalculado para intersectar las cajas.
5. **Render interactivo adaptativo**: mientras la cámara se mueve se renderiza a ⅓ de la resolución. Cuando se detiene, se hace un solo render completo, y si la cámara no se mueve no se vuelve a renderizar.
6. **Skybox precalculado**: las 6 caras se generan una vez al inicio. Cuando un rayo no choca con nada, solo se hace un muestreo bilineal.
7. **Luces con rango**: cada luz puntual tiene un radio de alcance. Fuera de ese radio no se lanza su rayo de sombra.
8. **Release**: `opt-level = 3`, LTO, `codegen-units = 1` y `target-cpu=native`.

## Estructura

```
src/
├── main.rs      ventana, input, modos --still y --video
├── math.rs      Vec3, reflect, refract, Schlick
├── ray.rs       Ray y AABB (slabs)
├── voxel.rs     cubo: normal, UV y marco tangente por cara
├── block.rs     tipos de bloque
├── world.rs     grilla 3D, DDA, sombras y oclusión ambiental
├── perlin.rs    ruido Perlin 2D/3D y fBm
├── texture.rs   texturas pixel-art 16×16 y normal maps generados por código
├── material.rs  parámetros de cada material
├── diorama.rs   construcción de las islas, el puente y las luces
├── light.rs     sol y luces puntuales
├── skybox.rs    cube map del atardecer
├── shading.rs   integrador recursivo (Whitted)
├── scene.rs     une mundo, materiales, luces y cielo
├── render.rs    render multihilo y tone mapping
├── camera.rs    cámara LookAt y orbital
├── video.rs     imagen fija y frames del video
└── image.rs     escritura de BMP/PPM byte a byte
```
