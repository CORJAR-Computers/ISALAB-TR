# Firebird 5 Embedded — librerías nativas

Este directorio se empaqueta como recurso de Tauri (`bundle.resources`) y se
carga en tiempo de ejecución por `rsfbclient` (feature `dynamic_loading`).

## Instalación (Windows)

1. Descarga **Firebird 5.0.x (64-bit) — ZIP** desde
   https://firebirdsql.org/en/firebird-5-0/ (o el instalador y copia la lib).
2. Copia **`fbclient.dll`** aquí, junto al resto del contenido de la carpeta
   `Windows` del ZIP:
   - `fbclient.dll`   ← obligatorio (el motor Embedded de Firebird 3+ es esta DLL)
   - `firebird.conf`  ← opcional (configuración del motor)
   - `security5.fdb`  ← opcional (solo si usas autenticación por contraseña)
   - `icudt*.dll`, `icuin*.dll`, `icuuc*.dll` ← solo si el build los requiere

> Con Embedded y autenticación de sistema (Win_Sspi) no hace falta
> `security5.fdb`. La app conecta con usuario `SYSDBA` sin contraseña (modo
> embedded), que es el comportamiento estándar de Firebird 3+.

## Linux / macOS

Coloca `libfbclient.so` / `libfbclient.dylib` aquí con el mismo nombre
(`fbclient.dll` se sustituye según plataforma en `state.rs` si es necesario;
por ahora el proyecto está orientado a Windows).

## Verificación

Al arrancar la app, si la librería falta, la UI muestra un banner con
instrucciones y `db_health` reporta `fbclientFound: false`. La base de datos
(`isalab.fdb`) se crea automáticamente en `app_data` al primer arranque
válido y las migraciones SQL se aplican solas.

> Los archivos `*.dll`, `*.so`, `*.fdb` están en `.gitignore`: no deben
> commitearse; el instalador de la app los incluye vía `bundle.resources`.
