# Openfoot Manager — Guía para Testers Alpha

¡Hola! Bienvenido al alpha de **Openfoot Manager**. Antes que nada — muchas gracias por estar aquí. En serio. El hecho de que estés dispuesto a jugar un juego sin terminar y ayudarnos a mejorarlo significa muchísimo para nosotros.

Esta guía te explicará qué es el juego, qué funciona, qué todavía no, y cómo puedes ayudarnos de la mejor manera.

---

## ¿Qué es Openfoot Manager?

Openfoot Manager es un **juego de simulación de gestión de fútbol open-source**. Piensa en él como una carta de amor al género clásico de football manager — te haces cargo de un club, gestionas tu plantilla, defines tácticas, manejas fichajes y guías a tu equipo a lo largo de una temporada completa de fútbol competitivo.

Está construido como una aplicación de escritorio usando [Tauri](https://tauri.app/) (backend en Rust + frontend en React), lo que significa que se ejecuta nativamente en Windows, macOS y Linux sin necesidad de navegador ni conexión a internet.

### Qué incluye este alpha

Esto es lo que puedes hacer ahora mismo:

- **Crear un entrenador** con tu nombre, fecha de nacimiento y nacionalidad
- **Elegir un equipo** de una liga generada con 16 equipos
- **Gestionar tu plantilla** — definir formaciones, elegir tu once titular, asignar lanzadores de balón parado
- **Entrenar a tus jugadores** — elegir enfoque de entrenamiento, intensidad y calendario semanal
- **Contratar y despedir staff** — entrenadores, fisioterapeutas, ojeadores, asistentes
- **Jugar partidos** — simulación minuto a minuto con controles tácticos, o delegar en tu asistente
- **Charlas de medio tiempo** y **ruedas de prensa post-partido** que afectan la moral
- **Leer las noticias** — informes de partidos, resúmenes de jornada, actualizaciones de clasificación
- **Gestionar tu bandeja de entrada** — mensajes del staff, directiva y eventos del juego
- **Observar jugadores** y **hacer fichajes**
- **Seguir las finanzas** — salarios, presupuestos, ingresos
- **Completar una temporada entera** y avanzar a la siguiente

### Qué NO incluye este alpha

Para alinear expectativas — estas son cosas planeadas pero aún no implementadas:

- Múltiples ligas / ascensos / descensos
- Competiciones de copa
- Negociación de contratos de jugadores
- Cantera / academia juvenil
- Tácticas detalladas (marcaje individual, jugadas a balón parado, etc.)
- Multijugador
- Sonido / música
- Tutorial / introducción guiada más allá de la primera semana

Llegaremos ahí. Pero ahora mismo, necesitamos tu ayuda para encontrar los bugs y las asperezas de lo que ya tenemos.

---

## Primeros Pasos

### Instalación

1. Descarga el instalador para tu plataforma desde el enlace que te compartimos
2. Ejecuta el instalador — en Windows puede aparecer una advertencia de SmartScreen ya que la app aún no está firmada digitalmente. Haz clic en "Más información" → "Ejecutar de todas formas"
3. Abre **Openfoot Manager**

### Tu primera partida

1. Haz clic en **New Game** en el menú principal
2. Rellena los datos de tu entrenador (nombre, fecha de nacimiento, nacionalidad)
3. Elige una base de datos de mundo (la opción "Random World" está bien)
4. Elige un equipo de la lista — revisa sus estadísticas y elige el que te parezca más divertido
5. Llegarás al **Dashboard** — este es tu centro de mando

### Cosas importantes que saber

- El botón **Continue** (arriba a la derecha) avanza el tiempo un día. La flecha desplegable al lado te permite saltar al día de partido.
- La **barra lateral** a la izquierda es tu navegación — Plantilla, Tácticas, Entrenamiento, Calendario, Finanzas, etc.
- Antes de un partido, elegirás cómo quieres jugarlo: **Go to the Field** (control total), **Watch as Spectator** (ver jugar a la IA), o **Delegate to Assistant** (resultado instantáneo).
- Durante los partidos, puedes hacer sustituciones, cambiar formación, cambiar estilo de juego y ajustar lanzadores de balón parado.
- Después de los partidos, hay una charla de medio tiempo/final y una rueda de prensa opcional.
- El juego **guarda automáticamente** al volver al menú principal. También puedes guardar manualmente desde los ajustes.

---

## Qué Necesitamos de Ti

### La versión corta

Juega el juego. Rompe todo. Cuéntanos qué pasó.

### La versión larga

Buscamos feedback en tres categorías:

#### 1. Bugs y crashes

Esta es la máxima prioridad. Si algo se rompe, crashea, se congela, o se comporta de una forma claramente incorrecta — queremos saberlo.

Ejemplos:
- El juego crashea cuando intento iniciar un partido
- Mis cambios de formación no se guardan entre sesiones
- Un jugador aparece como lesionado pero sigue en mi once titular
- La clasificación no cuadra después de la jornada 5
- Recibí un mensaje sobre un partido que aún no ha ocurrido

#### 2. Problemas de usabilidad

Cosas que no están rotas, pero son confusas, molestas o difíciles de usar.

Ejemplos:
- No pude descubrir cómo cambiar mi once titular
- La página de entrenamiento es abrumadora, no sé qué hace cada cosa
- El texto es demasiado pequeño / grande en mi pantalla
- No entiendo qué afecta realmente el "estilo de juego"
- La bandeja de entrada está llena de mensajes que no me importan

#### 3. Balance y sensación de juego

Esto es más subjetivo, pero igualmente valioso. ¿El juego *se siente* bien?

Ejemplos:
- Mi equipo gana todos los partidos 5-0, es demasiado fácil
- El entrenamiento no parece hacer ninguna diferencia
- La moral de los jugadores nunca cambia sin importar lo que haga
- Las ofertas de fichaje siempre son rechazadas
- Los equipos de la IA parecen demasiado débiles / fuertes

---

## Cómo Enviar Feedback

La forma más fácil de enviar feedback es a través de nuestros **templates de issue en GitHub**. Solo elige el correcto y rellénalo — **¡puedes escribir en tu idioma!**

Si quieres hablar con el equipo o con otros jugadores, también puedes unirte al servidor de Discord: https://discord.gg/2CXaesaukT

- [**Reporte de Bug**](https://github.com/openfootmanager/openfootmanager/issues/new?template=bug_report_es.yml) — Algo crasheó, se rompió o se comportó incorrectamente
- [**Feedback / Sugerencia**](https://github.com/openfootmanager/openfootmanager/issues/new?template=feedback_es.yml) — Problemas de usabilidad, balance o ideas
- [**Reporte de Sesión**](https://github.com/openfootmanager/openfootmanager/issues/new?template=session_report_es.yml) — Un resumen de tu sesión de juego (¡super valioso!)

### Archivos de log

Cuando reportes un bug, **por favor incluye tus archivos de log**. Contienen información detallada sobre lo que el juego estaba haciendo cuando algo salió mal, y muchas veces son la diferencia entre poder arreglar un bug en 5 minutos o pasar horas intentando reproducirlo.

**Dónde encontrar tus logs:**

- **Windows:** `C:\Users\<TuUsuario>\AppData\Roaming\com.sturdyrobot.openfootmanager\logs\`
- **macOS:** `~/Library/Application Support/com.sturdyrobot.openfootmanager/logs/`
- **Linux:** `~/.local/share/com.sturdyrobot.openfootmanager/logs/`

Simplemente comprime (zip) toda la carpeta `logs` y adjúntala a tu reporte. Los logs no contienen información personal — solo eventos del juego, comandos y trazas de errores.

---

## Problemas Conocidos

Estas son cosas que ya sabemos — no necesitas reportarlas (pero siéntete libre de comentar si afectan tu experiencia):

- **Sin sonido ni música** — el juego es completamente silencioso por ahora
- **El escalado de la interfaz** puede no ser perfecto en todos los tamaños de pantalla — revisa Configuración → Display → UI Scale si algo se ve raro
- **Parte del texto no está traducido** — la internacionalización es un trabajo en progreso
- **Los archivos de guardado de este alpha pueden no ser compatibles** con versiones futuras. ¡No te encariñes demasiado con tus partidas!
- **El rendimiento** puede bajar levemente al simular muchos días seguidos

---

## Unas Últimas Palabras

Este es un proyecto de pasión. No tiene detrás un gran estudio ni un presupuesto enorme. Lo hacen personas que aman el fútbol y los videojuegos, en su tiempo libre.

Tu feedback durante este alpha no es solo algo "bonito de tener" — está literalmente dando forma a lo que este juego se va a convertir. Cada bug que reportes, cada "esto me confundió", cada "¿no sería genial si..." — todo importa.

Así que juega, diviértete (¡eso esperamos!), y no te guardes el feedback. No hay preguntas tontas y ningún comentario es demasiado pequeño.

Gracias por ser parte de esto. Vamos a construir algo increíble juntos.

— El equipo de Openfoot Manager

---

*Versión alpha 0.2.0*
