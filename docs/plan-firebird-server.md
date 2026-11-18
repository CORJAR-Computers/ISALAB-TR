# ISALAB — Plan: Firebird 5 Server para 3 PCs

**Estado:** propuesto — pendiente de aprobación
**Base:** v1.1.0 (schema 0033)
**Objetivo:** pasar de **Firebird 5 Embedded** (una base local por PC) a
**Firebird 5 Server** (una base central, tres clientes Tauri por red).

---

## 1. Resumen ejecutivo

Hoy cada instalación de ISALAB crea y migra su propio `isalab.fdb` dentro de
`app_data` usando el motor embedded (`fbclient.dll` + `engine13.dll`). Con 3 PCs
eso produce **tres bases islas**: no comparten pacientes, muestras, facturas ni
inventario, y no hay forma soportada de unificarlas.

La solución es una topología **cliente/servidor**:

```text
PC 1 ─┐
PC 2 ─┼── TCP 3050 ──►  Firebird 5 Server (servicio Windows)
PC 3 ─┘                      │
                             ├─ alias ISALAB → C:\Firebird\data\isalab.fdb
                             ├─ usuario de app (sin DDL)
                             ├─ adjuntos/logos como BLOB (sin SMB)
                             └─ gbak programado + retención
```

El cambio de código es **acotado pero no trivial**: `rsfbclient` 0.27 ya soporta
`with_remote().host().port().user().pass()`, y los repositorios siguen usando
`SimpleConnection` sin reescribirse. Lo que hay que rediseñar es el **arranque**:
hoy cada cliente crea la base, aplica las 33 migraciones, siembra el admin y poda
`EVENT_LOG` en cada inicio. Con 3 PCs eso es una carrera de DDL y seeds.

**Decisión clave:** las migraciones dejan de ejecutarse desde cada cliente. La
app pasa a **verificar** la versión de schema; un actualizador administrativo
(comando/CLI) es el único que aplica DDL. Esto elimina la carrera sin necesitar
locks distribuidos.

**Aclaración sobre "migrar a Firebird 5":** la base actual ya fue creada por el
motor Firebird 5.0.3 (ODS **13.1**). No hay una migración de ODS pendiente: el
traspaso es de **topología** (embedded → server), no de formato. Un servidor
Firebird 5 abre el archivo tal cual, y **no hay vuelta atrás a Firebird 4** (FB4
solo abre ODS 13.0).

---

## 2. Diagnóstico del código actual (verificado)

| Pieza | Hoy | Consecuencia con 3 PCs |
| --- | --- | --- |
| Conexión | `builder_native().with_dyn_load(...).with_embedded()`, usuario `SYSDBA` sin contraseña | No hay red ni arbitraje central; el usuario no es válido en remoto |
| Base | `app_data/isalab.fdb`, se crea si falta | Cada PC tiene su propia base |
| Migraciones | 33 migraciones embebidas, aplicadas en **cada arranque** | 3 clientes compiten por `SCHEMA_MIGRATIONS` y DDL |
| Seed admin | `ensure_default_admin` en cada arranque | 3 UPDATE concurrentes sobre `USERS` |
| Mantenimiento | poda de `EVENT_LOG` en cada arranque | 3 DELETE redundantes sobre la misma tabla |
| Pool | 4 conexiones, `acquire()` **falla al instante** si está vacío | 12 conexiones + 6 listeners; sin espera ni timeout |
| Eventos | 2 listeners nativos por PC; payload = última fila de `EVENT_LOG` | Con 2 PCs escribiendo, el payload puede ser de otra fila |
| Health | `db_health` solo mira `init_error` + existencia de la DLL | Si el servidor cae, no hay banner accionable |
| Backup | ZIP de `app_data` local | No respalda la base autoritativa; copia una FDB viva |
| Bundle | `fbclient.dll` + `engine13.dll` + ICU + `SECURITY5.FDB` en `binaries/firebird/` | Runtime de motor, no de cliente remoto |
| CI | descarga el ZIP de FB 5.0.3 y poda a "embedded" | Sigue siendo válido para tests; el bundle de release debe cambiar |
| Archivos | adjuntos/logos/certificados con **rutas absolutas locales** en la BD | Una ruta creada en PC 1 no existe en PC 2 |
| Secretos | DPAPI ligado al usuario Windows | `groq_api_key`/`smtp_password` cifrados en PC 1 no se leen en PC 2 |

Archivos centrales:

- `src-tauri/src/db/mod.rs` — pool, `new_connection`, `create_database`, `bootstrap`.
- `src-tauri/src/db/migrations.rs` — runner sin lock.
- `src-tauri/src/state.rs` — resuelve `app_data/isalab.fdb` y `fbclient.dll`.
- `src-tauri/src/commands/db.rs` — `db_health`, `create_local_backup`.
- `src-tauri/src/db/events.rs` — listeners `POST_EVENT`.
- `src-tauri/src/db/seed.rs`, `maintenance.rs` — seed y poda por arranque.
- `src-tauri/tauri.conf.json` — bundle `binaries/firebird/**/*`.
- `scripts/prune-firebird.mjs` — poda a subconjunto embedded.
- `src/App.tsx` — banner "Motor Firebird Embedded no encontrado".

---

## 3. Hechos de Firebird 5 que condicionan el plan

Verificados contra la distribución 5.0.3 incluida en el repo y las notas de
release oficiales de Firebird 5.0.4:

1. **ODS 13.1.** Firebird 5 introduce ODS 13.1. Las bases ODS 13.0 (Firebird 4)
   se pueden abrir, pero no al revés. La base de ISALAB ya es 13.1.
   - No hace falta `gbak`/restore por ODS.
   - `gfix -upgrade` solo aplica a bases 13.0 y es **irreversible** (no lo
     necesitamos).
2. **Mismo motor, mismo `engine13.dll`.** Un servidor FB5 usa el mismo formato y
   la misma librería de motor; el archivo `.fdb` es portable entre embedded y
   server FB5.
3. **Proveedores.** `Providers = Remote, Engine13, Loopback` (default). Con
   `host/port` explícitos se usa `Remote`. En el **cliente** se puede fijar
   `Providers = Remote` para impedir que la app abra archivos locales.
4. **Cadena de conexión.** `host/port:ruta_o_alias` (p. ej.
   `192.168.1.10/3050:ISALAB`). El alias se define en `databases.conf` del
   servidor.
5. **Eventos remotos.** `POST_EVENT` funciona por red con el cliente nativo
   (`listen_event` usa la API nativa; `pure_rust` no soporta eventos). El
   servidor entrega eventos por el **puerto auxiliar**: `RemoteAuxPort = 0`
   significa "puerto aleatorio". Si el firewall solo abre 3050, los eventos
   pueden no llegar → **fijar `RemoteAuxPort` y abrirlo**.
6. **Seguridad.** En remoto no existe el "SYSDBA sin contraseña" de embedded.
   Hay que crear un usuario de aplicación y usar SRP; el default de wire crypt
   (`ChaCha64`) ya cifra el canal.
7. **Instalación Windows.** El ZIP trae `firebird.exe`, `fbguard.exe`,
   `instsvc.exe`, `gbak`, `gfix`, `isql` y `install_service.bat`. El servicio se
   registra con `install_service.bat` (super-server + auto + guardian).
8. **Backups.** `gbak -b` (lógico, transportable) y `nbackup` (físico,
   incremental). Para el tamaño de una clínica, `gbak` diario + retención es
   suficiente; `nbackup` es opcional si la base crece.
9. **No compartir el `.fdb` por SMB.** Tres motores embedded sobre una carpeta
   compartida corrompen la base: no hay lock manager central. La única forma
   soportada es un servidor Firebird.
10. **Zona horaria.** El servidor usa la hora del servidor; el código actual
    interpreta timestamps como hora local del cliente
    (`sources/watched_folder.rs`). Con una sola clínica y una sola zona horaria
    no es problema, pero debe documentarse.

---

## 4. Arquitectura objetivo

### 4.1 Servidor

- Firebird **5.0.4 x64** (misma rama que el cliente; 5.0.3 también sirve).
- Servicio Windows con arranque automático + `fbguard` (reinicio ante caída).
- Un único `.fdb` autoritativo en disco local del servidor (nunca en SMB).
- Alias en `databases.conf`:

  ```ini
  ISALAB = C:\Firebird\data\isalab.fdb
  ```

- `RemoteAccess = true` (solo para `ISALAB`; la security DB se restringe en
  `databases.conf`).
- `RemoteAuxPort = 3051` (eventos) y `RemoteServicePort = 3050`.
- Firewall: entrada TCP **3050** y **3051** solo desde las IP de las 3 PCs,
  perfil de red privada. Nada expuesto a Internet.
- `DatabaseAccess = Restrict C:\Firebird\data` (defensa en profundidad para
  accesos locales/embedded).
- **Sin share SMB:** adjuntos, logos y certificado se guardan como **BLOB** en
  la misma base. El único requisito de red es TCP 3050/3051, no la compartición
  de archivos de Windows.
- Backups programados con `gbak` (incluye los BLOB) + copia externa.

### 4.1.1 Hardware y sistema operativo del servidor

**Hardware: sí, un Tiny con i5/i7 de 7.ª generación en adelante sobra** para 3
clientes de una clínica. Firebird 5 es muy liviano y el cuello de botella real
será la red y el disco, no la CPU. Configuración mínima recomendada:

| Componente | Mínimo | Recomendado |
| --- | --- | --- |
| CPU | i5 de 7.ª gen (4 núcleos) | i5/i7 de 8.ª gen+ o Intel N100/N305 |
| RAM | 8 GB | 16 GB |
| Disco | SSD 128 GB | SSD/NVMe 256 GB+ |
| Red | Ethernet 1 Gbps | Ethernet 1 Gbps cableado |
| Energía | — | UPS pequeña |
| SO | Windows 11 Pro (64-bit) | Windows 11 Pro o Linux LTS |

**Decisión (2026-11-18):** se comprará un Tiny **compatible con Windows 11**
(8.ª generación o superior, o Intel N100/N305) con **Windows 11 Pro**. No se
usará el Tiny de 7.ª generación con Windows 10.

**Advertencia importante sobre el sistema operativo:** un i5/i7 de **7.ª
generación no es compatible oficialmente con Windows 11** (Microsoft exige 8.ª
generación o superior) y **Windows 10 dejó de recibir soporte en octubre de
2025**. Para un equipo que guarda historias clínicas, no conviene un SO sin
parches. Opciones, en orden de preferencia:

1. **Linux LTS** (Ubuntu Server 24.04 / Debian 12) + Firebird 5 + Samba para los
   adjuntos. El cliente sigue siendo Windows; el servidor puede ser Linux. Es la
   opción más segura y gratuita si el Tiny es de 7.ª generación.
2. **Mini-PC nuevo con Windows 11 Pro** (8.ª gen+ o N100/N305, 16 GB, SSD). Es
   la opción más simple si el equipo que administra la clínica solo maneja
   Windows.
3. Windows 10/11 "no soportado" en el Tiny de 7.ª gen: **no recomendado** para
   datos clínicos.

Sea Windows o Linux, el servidor debe ser **dedicado** (no una de las 3 PCs de
trabajo), con arranque automático del servicio, sin suspensión y con UPS. Si el
"servidor" es una PC de trabajo, apagarla o actualizarla tumba la clínica.

### 4.2 Clientes

- Se instala el mismo NSIS en las 3 PCs.
- Cada PC guarda su configuración de conexión (host, puerto, alias, usuario y
  secreto protegido con DPAPI).
- `fbclient.dll` + plugins de autenticación/cifrado + `intl` + ICU (runtime de
  **cliente**, no de motor).
- `firebird.conf` del cliente con `Providers = Remote` y sin `engine13.dll`
  (evita abrir bases locales por accidente).
- Pool de 4 conexiones por PC + 2 listeners ⇒ ~18 attachments al servidor. Es
  trivial para Firebird; no requiere tuning especial.

### 4.3 Lo que **no** cambia

- `rsfbclient` y `dynamic_loading`.
- `SimpleConnection` y prácticamente todos los repositorios.
- React Query, comandos Tauri, bindings y UI de dominio.
- Las migraciones 0001–0033 ya publicadas (no se reescriben).

---

## 5. Decisiones de infraestructura

| Decisión | Recomendación | Alternativa |
| --- | --- | --- |
| Dónde corre el servidor | Tiny dedicado en la red local | Un PC de trabajo (riesgo de caída) |
| SO del servidor | Linux LTS si el equipo es de 7.ª gen; Windows 11 Pro si es 8.ª gen+ | Windows 10 (sin soporte desde oct-2025) |
| Arquitectura FB | SuperServer (default) | Classic (no aporta nada para 3 clientes) |
| Identificación | IP fija + alias `ISALAB` | Hostname DNS local (más frágil) |
| Usuario de la app | `ISALAB_APP` con permisos DML, sin DDL | `SYSDBA` (rechazado: demasiado privilegio) |
| Migraciones | Runner administrativo único | Lock distribuido en la app (más complejo) |
| Datos iniciales | **Base limpia** (decidido: negocio nuevo) | — |
| Archivos | **BLOB en Firebird** (sin SMB) | Carpeta compartida SMB con rutas relativas |
| Secretos de app | Compartidos, cifrados con clave maestra de clínica | Configuración independiente por PC |
| Backups | `gbak` diario (incluye los BLOB) + retención | `nbackup` incremental |
| Contraseña de BD | DPAPI por PC, editable en Ajustes | Variable de entorno (menos usable) |

---

## 6. Plan por fases

Cada fase termina con verificación y **un commit** (conventional commit en
inglés), siguiendo el flujo del proyecto.

### Fase 0 — Preparación y línea base (sin código)

**Objetivo:** cerrar las decisiones de infraestructura antes de instalar.

Tareas:

1. Confirmar el equipamiento (Tiny i5/i7 7.ª gen+) y su sistema operativo
   (Linux LTS o Windows 11 Pro; ver §4.1.1).
2. Reservar IP fija para el servidor y definir la ventana de corte.
3. Definir nombres: alias `ISALAB`, usuario `ISALAB_APP`, carpeta de datos
   `C:\Firebird\data` (o `/var/lib/firebird/data` en Linux).
4. Confirmar que **no hay datos previos que migrar**: la clínica arranca en cero.
   Las bases de desarrollo con datos demo no se trasladan.
5. Confirmar la política de backups (frecuencia, retención, copia externa) y el
   responsable de ejecutar el runner de migraciones.

**Entregable:** este documento aprobado + checklist de aprovisionamiento.

**Verificación:** hardware, SO, IP, nombres y responsable de backups definidos.

---

### Fase 1 — Servidor Firebird 5 operativo

**Objetivo:** servidor instalado, con base, usuario y backups, antes de tocar la
app.

Tareas:

1. Instalar Firebird 5.0.4 x64 (ZIP o instalador) en `C:\Firebird\Firebird_5_0`.
2. Registrar el servicio:

   ```bat
   cd /d "C:\Firebird\Firebird_5_0"
   install_service.bat
   ```

   (o `instsvc install -auto -superserver -guardian`). Verificar con
   `instsvc query` y `Get-Service`.)
3. Crear carpeta de datos `C:\Firebird\data` y dar permisos a la cuenta del
   servicio (LocalSystem por defecto, o cuenta dedicada con `-login`).
4. Configurar `firebird.conf` del servidor:
   `RemoteServicePort = 3050`, `RemoteAuxPort = 3051`,
   `DatabaseAccess = Restrict C:\Firebird\data`,
   `WireCryptPlugin = ChaCha64, ChaCha, Arc4` (default).
5. `databases.conf`: alias `ISALAB` y denegar acceso remoto a `security5.fdb`.
6. Firewall: reglas de entrada TCP 3050/3051 limitadas a las IP de las 3 PCs.
7. Crear la base vacía (la Fase 2 detalla página y charset):

   ```sql
   CREATE DATABASE 'C:\Firebird\data\isalab.fdb'
     PAGE_SIZE 16384 DEFAULT CHARACTER SET UTF8
     USER 'SYSDBA' PASSWORD '<clave>';
   ```
8. Crear el usuario de aplicación (sin grants todavía: el esquema aún no
   existe):

   ```sql
   CREATE USER ISALAB_APP PASSWORD '<clave>' USING PLUGIN Srp;
   ```

   **Trampa:** los grants se aplican **después** del runner (Fase 4), porque
   `GRANT ... ON ALL TABLES` es una foto del momento: las tablas que creen
   futuras migraciones no heredan el permiso y hay que volver a otorgarlo.
9. Backup programado (Programador de tareas de Windows, cuenta con acceso a la
   carpeta):

   ```bat
   gbak -b -t -v -user SYSDBA -password <clave> ^
        localhost/3050:ISALAB C:\Firebird\backups\isalab_%DATE%.fbk
   ```

   Retención 30 días + copia a otro disco/NAS.
10. Probar desde otra PC:

    ```bat
    isql -user ISALAB_APP -password <clave> 192.168.1.10/3050:ISALAB
    ```

**Entregable:** servidor documentado + `deploy/firebird/databases.conf.example`
y `scripts/verify-firebird-remote.ps1` (nuevos).

**Verificación:** `isql` remoto autentica y responde `SELECT 1 FROM RDB$DATABASE`;
`gstat -h` muestra ODS 13.1; el backup se genera y **se restaura** en una base de
prueba.

---

### Fase 2 — Base limpia en el servidor

**Objetivo:** crear la base vacía en el servidor y probar el circuito de
backup/restore **antes** de que haya datos. La clínica arranca de cero: no se
migran bases de desarrollo ni datos demo.

Tareas:

1. Crear la base una sola vez, como `SYSDBA`, con página y charset explícitos
   (el runner y la app asumen UTF8):

   ```sql
   CREATE DATABASE 'C:\Firebird\data\isalab.fdb'
     PAGE_SIZE 16384
     DEFAULT CHARACTER SET UTF8
     USER 'SYSDBA' PASSWORD '<clave>';
   ```

2. Verificar el ODS y el charset:

   ```sql
   SELECT RDB$ODS_VERSION, RDB$CHARACTER_SET_NAME FROM RDB$DATABASE;
   ```

3. Ensayo de backup/restore en vacío:

   ```bat
   gbak -b -t -v localhost/3050:ISALAB C:\Firebird\backups\drill.fbk
   gbak -c -r -v C:\Firebird\backups\drill.fbk ^
        C:\Firebird\data\isalab_restore_drill.fdb
   gfix -validate C:\Firebird\data\isalab_restore_drill.fdb
   ```

   Abrir la copia con `isql`, confirmar que responde y borrarla.

4. Dejar documentado el procedimiento de backup/restore en
   `docs/firebird-migration-runbook.md` (aunque hoy no haya datos que migrar:
   el mismo runbook se usa para restaurar un backup real).

5. **No** copiar `isalab.fdb` desde ninguna PC de desarrollo. Si alguna vez se
   quisiera conservar una base existente, el anexo del runbook describe el
   `gbak -b` / `gbak -c -r`; no es el caso de esta puesta en marcha.

**Entregable:** base `isalab.fdb` vacía + runbook de backup/restore.

**Verificación:** `gstat -h` muestra ODS 13.1; el round-trip backup → restore →
`gfix -validate` es limpio; `isql` como `ISALAB_APP` conecta (aún sin tablas, el
login debe funcionar).

**Riesgo:** crear la base con charset o page size distintos a los esperados
obliga a recrearla; por eso los parámetros van explícitos.

---

### Fase 3 — Capa de conexión remota en Rust

**Objetivo:** la app abre la base del servidor, sin migrar ni sembrar.

Tareas:

1. Nuevo `src-tauri/src/db/config.rs`:

   ```rust
   pub enum DbMode { Embedded, Remote }

   pub struct DbConfig {
       pub mode: DbMode,
       pub host: String,       // "192.168.1.10"
       pub port: u16,          // 3050
       pub database: String,   // "ISALAB" (alias) o ruta del servidor
       pub user: String,       // "ISALAB_APP"
       pub secret_ref: SecretRef, // DPAPI / variable de entorno
       pub connect_timeout: Duration,
   }
   ```

2. Resolución de configuración por prioridad:
   variables de entorno (`ISALAB_DB_HOST`, `ISALAB_DB_PORT`, `ISALAB_DB_NAME`,
   `ISALAB_DB_USER`, `ISALAB_DB_PASSWORD`) → archivo
   `%APPDATA%\com.isalab.clinic\db-connection.json` → asistente de primera
   ejecución. La contraseña **nunca** en texto plano; reutilizar
   `src-tauri/src/crypto.rs` (DPAPI) para el blob local.

3. `db/mod.rs`:
   - `new_connection` usa `.with_remote().host().port().pass().db_name()`.
   - `create_database` solo se usa en tests/embedded; en remoto la base la crea
     el administrador.
   - `bootstrap` deja de crear/migrar/sembrar: abre una conexión, **consulta**
     `MAX(VERSION)` y monta el pool.
   - Pool con `Condvar`/semáforo y timeout (p. ej. 5 s) en vez de fallar al
     instante; reconexión ante conexiones muertas (una conexión devuelta por un
     servidor reiniciado debe descartarse, no reutilizarse).
   - Timeout de conexión (no bloquear el splash indefinidamente).

4. `state.rs`: guarda `DbConfig`, `db_target` (host:puerto/alias) y un estado de
   salud tipado; elimina la resolución de `app_data/isalab.fdb` en remoto.

5. `commands/db.rs`: `db_health` hace un **ping real** (`SELECT 1 FROM
   RDB$DATABASE`) con timeout y devuelve un enum:

   ```text
   CLIENT_MISSING | SERVER_UNREACHABLE | AUTH_FAILED |
   SCHEMA_TOO_OLD | SCHEMA_TOO_NEW | CONNECTED
   ```

   Incluye `mode`, `host`, `port`, `database`, `schemaVersion`, `latencyMs`.

6. `src/App.tsx` + `use-db-health.ts`: banner accionable por estado (servidor
   inaccesible → "Reintentar" + IP/puerto; schema desactualizado → "Ejecute el
   actualizador"; credenciales → "Revisar configuración"). Quitar el texto de
   `fbclient.dll` como causa principal.

7. `LoginPage.tsx` y `AboutDialog.tsx`: cambiar "datos 100 % locales /
   Firebird Embedded" por "Firebird 5 Server · red local".

**Entregable:** commit `feat(db): connect to Firebird 5 server over TCP`.

**Verificación:**

- `cargo test` (los tests embedded siguen verdes con `DbMode::Embedded`).
- `npm test`, `tsc`, `lint`, `build`.
- Smoke manual: app en PC 1 contra el servidor; PC 2 y PC 3 también.
- Servidor apagado → banner correcto, sin cuelgue del splash.
- Servidor reiniciado → la app se recupera sin reinstalar.

**Riesgo:** el pool actual no tolera conexiones muertas; sin reconexión, cada
caída del servidor exige reiniciar la app. Es parte del alcance de esta fase.

---

### Fase 4 — Migraciones y seed administrados (una sola vez)

**Objetivo:** eliminar la carrera de DDL entre 3 clientes.

Tareas:

1. La app **no** aplica migraciones en producción. Si la versión del schema es
   menor que la esperada, muestra "Base de datos desactualizada" y no opera.
2. Nuevo comando/CLI administrativo, ejecutado una sola vez desde el servidor o
   una PC de administración:

   ```text
   isalab.exe --migrate --host 192.168.1.10 --database ISALAB --user SYSDBA
   ```

   (o un binario `src-tauri/src/bin/migrate.rs`). Reutiliza
   `migrations::run_migrations` y `seed::ensure_default_admin`.
3. `run_migrations` se ejecuta dentro de una **transacción por migración**
   (DDL es transaccional en Firebird) y registra `SCHEMA_MIGRATIONS` en la misma
   transacción: si falla, no queda a medias.
4. `ensure_default_admin` se mueve al runner (un único UPDATE, no 3).
5. Tras migrar, el runner vuelve a aplicar los grants a `ISALAB_APP`.
6. `maintenance::prune_event_log` se mueve a una tarea programada del servidor
   (o se ejecuta solo en el runner). Los clientes no podan.
7. `should_seed_demo_data()` sigue devolviendo `true` en debug; añadir guardia
   explícita: si el destino no es `localhost`, **rechazar** el seed demo. Un
   debug conectado a la base de la clínica no debe insertar datos de prueba.
8. Bloqueo de seguridad: el runner registra la versión aplicada; los clientes
   llevan la versión esperada compilada y comparan. Si `schema < esperado` →
   bloquear mutaciones; si `schema > esperado` → avisar de app desactualizada.

**Entregable:** commit `feat(db): run schema migrations from an admin runner`.

**Verificación:**

- Arrancar 3 clientes en paralelo contra una base vacía: **ninguno** migra; los
  3 muestran el estado "desactualizado" hasta que corre el runner.
- Runner aplica 0001–0033 + grants en una sola pasada; los 3 clientes pasan a
  `CONNECTED`.
- Fallo simulado en una migración → no queda versión registrada ni objetos
  parciales.
- Test Rust: dos migraciones concurrentes no pueden registrarse dos veces.

**Riesgo:** un despliegue con migraciones nuevas exige coordinar actualización
de app + runner. Documentarlo en `docs/release-process.md` (ventana de
mantenimiento).

---

### Fase 5 — Eventos, mantenimiento y concurrencia

**Objetivo:** que 3 PCs se vean en vivo y no se pisen.

Tareas:

1. `events.rs`:
   - Suscriptor con reconexión + backoff y logs (hoy `unwrap_or_default()`
     oculta el fallo).
   - Payload correlacionado: `POST_EVENT` no lleva datos; en vez de "última fila
     de `EVENT_LOG`", leer por `EVENT_LOG.ID` capturado antes del `POST_EVENT`
     (o incluir un `EVENT_LOG` con `REF_ID`+`STATUS` y filtrar por
     `PATIENT_ID`/`REF_ID` en el cliente). Si no se puede, el handler debe
     emitir un evento **genérico** de invalidación (la UI refetchea) en lugar de
     un payload potencialmente incorrecto.
   - Verificar eventos con 2 PCs: PC A crea muestra → PC B recibe
     `sample-changed` sin recargar.
2. `RemoteAuxPort` fijo (3051) + regla de firewall; probar con el firewall
   activo en las 3 PCs.
3. Transacciones en operaciones multipaso (riesgo real de datos parciales):
   - `visits` (visita + servicios), `invoices` (factura + ítems),
     `appointments` (validar solapamiento + insertar),
     `stays` (capacidad + insertar), `products` (movimiento + validación de
     stock), importaciones por lote.
   - Usar `begin_transaction`/`commit`/`rollback`; nunca devolver al pool una
     conexión con transacción abierta.
4. Condiciones de carrera "SELECT y luego INSERT":
   - Citas solapadas: validar dentro de la transacción y serializar por
     profesional (tabla de lock o `UPDATE` de un contador con `WITH LOCK`), o
     aceptar el conflicto y reintentar.
   - Capacidad de estancias: contar y insertar en la misma transacción.
   - Transiciones de estado: `UPDATE ... WHERE STATUS = <esperado>` y verificar
     filas afectadas (compare-and-set) en vez de leer y luego escribir.
5. `sources::start_supervisor`: si las carpetas vigiladas se comparten entre
   PCs, añadir claim/lease por archivo (tabla `ANALYZER_IMPORT_JOBS` ya existe)
   para que dos PCs no procesen el mismo archivo. Si cada carpeta es local a una
   PC, documentarlo y no compartirla.
6. `commands/ai.rs`: liberar la conexión del pool **antes** de la llamada HTTP
   (hoy puede retenerla 30–60 s).

**Entregable:** commit `feat(db): remote events and concurrent-safe operations`.

**Verificación:** pruebas de 2–3 PCs simultáneas sobre citas, estancias,
facturas y stock; sin duplicados ni sobrecupo; eventos cruzados en < 1 s.

---

### Fase 6 — Archivos, secretos y backup

**Objetivo:** que la base central no apunte a archivos locales de una PC.

**Decisión revisada (2026-11-18): BLOB en Firebird, sin SMB.**

El cliente tuvo problemas de red con Windows 11: el servidor existía, pero no se
podía acceder a sus recursos compartidos desde los otros PCs. Ese problema es de
**SMB / descubrimiento de red / firewall**, no de Firebird. Para no depender de
SMB en absoluto, los adjuntos, logos y el certificado se guardan como **BLOB**
en la misma base:

- Un solo protocolo (TCP 3050) y un solo backup (`gbak`, que incluye los BLOB).
- Sin share, sin ACL de red, sin descubrimiento, sin cuentas locales de share,
  sin `net use`.
- Consistencia total: el archivo y su fila se escriben en la misma transacción.
- `rsfbclient` 0.27 soporta BLOB binarios (`Vec<u8>`) y de texto, incluidos los
  grandes (probado en la suite del crate).

**Coste:** la base crece. Una clínica con fotos de peluquería y PDFs de
resultados puede generar unos pocos GB al año; es manejable. Si el crecimiento
se dispara, se pasa a `nbackup` incremental y/o a archivos fuera de la base.

Tareas:

1. Migración nueva (p. ej. `0034_attachment_blobs.sql`) con:
   - `ATTACHMENTS.CONTENT BLOB SUB_TYPE 0` (binario) + `MIME_TYPE`,
     `FILE_NAME`, `SIZE_BYTES`.
   - `VISIT_ATTACHMENTS.CONTENT` equivalente.
   - `LOGOS.CONTENT` (o una tabla `CLINIC_ASSETS` con `KIND` =
     `LOGO | LOGO_SECONDARY | CERT_P12`).
   - Mantener las columnas de ruta solo si se quieren para exportar; la fuente
     de verdad es el BLOB.
2. Reemplazar los comandos que devuelven `file_path` por:
   - `get_attachment(id) -> Vec<u8>` (el backend lee el BLOB y lo entrega al
     webview; para `<img>` se puede devolver un `data:` URL o escribir una caché
     temporal).
   - `import_attachment(path)` lee el archivo local una vez y lo inserta como
     BLOB.
   - Los reportes PDF generados localmente siguen siendo locales: se regeneran
     cuando hace falta y no necesitan compartirse.
3. Certificado PKCS#12: BLOB con acceso solo ADMIN; al firmar, el backend lo
   escribe en un archivo temporal con permisos restringidos, firma y lo borra.
   La contraseña se pide en memoria y no se persiste.
4. Sanear tamaño y tipo MIME antes de insertar (límite configurable, p. ej.
   20 MB por adjunto) y validar que el BLOB se lee completo.
5. Secretos compartidos:
   - Generar una **clave maestra de clínica** (32 bytes) una sola vez.
   - En cada PC, guardarla envuelta con DPAPI en
     `%APPDATA%\com.isalab.clinic\secrets.key` (nunca en la base).
   - Cifrar los valores con AEAD (ChaCha20-Poly1305 o AES-256-GCM) antes de
     escribirlos en `CLINIC_SETTINGS`; `crypto.rs` pasa de "DPAPI por valor" a
     "clave maestra + AEAD".
   - Al ser base limpia, no hay secretos legacy que migrar.
   - El certificado ya no vive en un share: se guarda como BLOB (tarea 3).
6. `create_local_backup`: renombrar a "exportar configuración local" (conexión,
   preferencias) y añadir en Ajustes el estado del backup del servidor (ruta y
   fecha del último `gbak`), sin intentar copiar la FDB viva por red.
7. `db_health`/UI: mostrar "Servidor: 192.168.1.10/3050 · ISALAB · schema
   v0033" en lugar de una ruta local.
8. Backup: `gbak` cubre datos **y** BLOB. Verificar el restore con al menos un
   adjunto y un logo; opcionalmente, un comando de exportación a disco para
   inspección manual (no es la fuente de verdad).

**Entregable:** commit `feat(files): store attachments as Firebird BLOBs`.

**Verificación:** adjuntar en PC 1 y abrir en PC 2; logo visible en las 3;
backup del servidor restaurado en una base de prueba **con los adjuntos
intactos**; config local exportable.

---

### Fase 7 — Empaquetado, CI y documentación

**Objetivo:** el instalador deja de llevar un motor embedded y pasa a ser un
cliente.

Tareas:

1. `tauri.conf.json`: renombrar el recurso a `binaries/firebird-client/**/*` y
   actualizar `longDescription` ("Firebird 5 Server").
2. `scripts/prune-firebird.mjs`: subconjunto **cliente** (fbclient + plugins de
   auth/crypt + intl + ICU + VC runtime + `firebird.conf`). Eliminar
   `engine13.dll` y `SECURITY5.FDB` del bundle cliente (validar antes con el
   smoke test remoto: si SRP los necesita, conservar `SECURITY5.FDB`). Validar
   `ESSENTIAL` acorde.
3. `firebird.conf` del cliente: `Providers = Remote` (sin `Engine13`), para que
   la app no abra archivos locales.
4. `firebird.conf.example`: documentar la variante cliente y la de servidor por
   separado (hoy dice `Providers = Embedded, Local`, que desactivaría Remote).
5. CI (`.github/workflows/ci.yml`, `release.yml`): los tests Rust siguen usando
   el runtime embedded descargado (`FIREBIRD_DIR`); el bundle de release usa el
   directorio cliente. Añadir job opcional "smoke remoto" con un servicio FB5
   en Windows.
6. `src-tauri/src/test_helpers.rs`: mantener tests embedded; añadir
   `setup_remote_test_db()` solo si hay servidor (ignorado por defecto).
7. Documentación: `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`,
   `docs/release-process.md`, `docs/plan-clinica-veterinaria-integral.md` y
   `docs/roadmap-v0.6.0.md`. Nuevos: `docs/firebird-server-deployment.md`,
   `docs/firebird-client-configuration.md`,
   `docs/firebird-migration-runbook.md`.
8. `deploy/firebird/` con `databases.conf.example`, `firebird.conf.server`,
   `backup-isalab.ps1`, `verify-firebird-remote.ps1`, `README.md`.

**Entregable:** commit `build: ship Firebird client runtime and server docs`.

**Verificación:** instalador limpio en una PC sin Firebird instalado → conecta
al servidor; `npm run tauri:build` verde; CI verde; el bundle ya no contiene
`engine13.dll`.

---

### Fase 8 — Corte, QA y puesta en producción

**Objetivo:** salir de embedded sin pérdida de datos.

Tareas:

1. Congelar las 3 PCs, backup final, restore en el servidor (Fase 2).
2. Configurar las 3 PCs con `db-connection.json` + contraseña DPAPI.
3. Matriz de QA (ver §10) en 2 y 3 PCs.
4. Capacitación breve: qué hacer si "servidor no disponible"; no apagar el
   servidor; no copiar el `.fdb`; backups.
5. Plan de rollback (ver §9).
6. Monitoreo de la primera semana: `firebird.log`, `MON$ATTACHMENTS`,
   deadlocks, latencia.

**Entregable:** commit `docs(ops): firebird server cutover runbook`.

**Verificación:** clínica operando 5 días con 3 PCs + backup restaurado.

---

## 7. Cambios de código previstos (resumen)

| Archivo | Cambio |
| --- | --- |
| `src-tauri/src/db/mod.rs` | Remote builder, pool con timeout/reconexión, bootstrap sin DDL |
| `src-tauri/src/db/config.rs` *(nuevo)* | `DbMode`, `DbConfig`, prioridad env/archivo, DPAPI |
| `src-tauri/src/db/migrations.rs` | Transacción por migración, sin auto-run en clientes |
| `src-tauri/src/db/seed.rs` | Seed solo desde runner; guardia anti-producción |
| `src-tauri/src/db/maintenance.rs` | Poda en servidor/runner, no en cada PC |
| `src-tauri/src/db/events.rs` | Reconexión, logs, payload correlacionado |
| `src-tauri/src/state.rs` | `DbConfig` + health tipado; sin path local de DB |
| `src-tauri/src/commands/db.rs` | Ping real + estados; backup server-side |
| `src-tauri/src/lib.rs` | Arranque no bloqueante, timeout de conexión |
| `src-tauri/src/bin/migrate.rs` *(nuevo)* | Runner de migraciones + grants |
| `src-tauri/src/repositories/appointments.rs` | Carrera de solapamiento |
| `src-tauri/src/repositories/stays.rs` | Carrera de capacidad |
| `src-tauri/src/repositories/visits.rs` | Transacción visita+servicios, CAS de estado |
| `src-tauri/src/repositories/invoices.rs` | Transacción factura+ítems, CAS de estado |
| `src-tauri/src/repositories/products.rs` | Movimiento de stock atómico |
| `src-tauri/src/commands/ai.rs` | Liberar conexión antes del HTTP |
| `src-tauri/src/sources/*` | Claim por archivo si se comparten carpetas |
| `src-tauri/src/commands/attachments.rs`, `grooming.rs`, `settings.rs` | BLOB en Firebird (`get_attachment`, `import_attachment`) |
| `src-tauri/migrations/0034_*.sql` *(nueva)* | Columnas `CONTENT BLOB`, MIME, nombre y tamaño |
| `src-tauri/tauri.conf.json` | Recurso `firebird-client`, descripción |
| `scripts/prune-firebird.mjs` | Subconjunto cliente |
| `src-tauri/binaries/firebird/firebird.conf` | `Providers = Remote` en cliente |
| `.github/workflows/ci.yml`, `release.yml` | Bundle cliente + smoke remoto |
| `src/App.tsx`, `use-db-health.ts`, `LoginPage.tsx`, `AboutDialog.tsx` | Banners y textos |
| `src/features/settings/SettingsPage.tsx` | Config de conexión + backup |
| `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `docs/*` | Documentación |

**No se tocan:** repositorios query-only, React Query, UI de dominio, migraciones
0001–0033, `Cargo.lock` (salvo que se añada una dependencia).

---

## 8. Puesta en marcha de datos (base limpia)

**Decidido:** la clínica inicia operaciones desde cero. No hay bases previas ni
datos que fusionar, así que desaparece el riesgo de migración y el de bases
divergentes.

Secuencia:

1. Crear `isalab.fdb` vacía en el servidor (Fase 2).
2. Ejecutar el runner → aplica 0001–0033. En build de release, la migración 0005
   (datos demo) se registra **sin ejecutar sus INSERT**.
3. `ensure_default_admin` fija `admin/admin123` con `MUST_CHANGE_PASSWORD`.
4. Crear los usuarios del personal (ADMIN/VET/AUX/RECEPCION) desde la UI.
5. Cargar catálogo de servicios, precios, configuración de la clínica y logo.
6. Configurar las 3 PCs y hacer el smoke test.
7. Backup inicial (`gbak` + archivos) y **restore drill** antes de abrir.

> La base de desarrollo con datos demo **no** se copia al servidor. Si se
> necesitan datos de ejemplo para capacitación, se usa una base de prueba local
> con `ISALAB_SEED_DEMO=1`, nunca la base de la clínica.

El `.fdb` del servidor es la **única** fuente de verdad. Nunca se copia una FDB
desde una PC cliente.

---

## 9. Corte y rollback

**Corte (ventana de 1–2 h, antes de la apertura):**

1. Servidor instalado y base creada (Fases 1–2).
2. Runner aplicado + usuario admin + catálogo cargado (Fase 4).
3. Instalar la versión cliente en las 3 PCs y configurar la conexión.
4. Smoke test: login, paciente, muestra, factura, estancia, producto.
5. Prueba cruzada: PC A crea una muestra, PC B la ve sin recargar.
6. Backup inicial + restore drill.
7. Abrir la clínica.

Como no hay datos previos, no hace falta congelar bases ni restaurar un backup
de producción: el "corte" es una puesta en marcha.

**Rollback (si algo falla antes de abrir):**

1. Cerrar las apps nuevas.
2. Corregir el servidor/configuración y reintentar; la base está vacía o solo
   tiene datos de prueba.
3. Si hubiera que volver a embedded temporalmente, se puede reinstalar la
   versión anterior y empezar de cero (no hay datos productivos que rescatar).

**Si el fallo ocurre ya en operación:** exportar la base con `gbak`, conservarla
y corregir la causa. Volver a embedded **no** es una opción de rollback limpia:
habría que importar el esquema y los datos a una base local con un script
(mismo ODS, pero cambia la topología). Por eso el corte se hace con la clínica
cerrada y con el smoke test completo.

---

## 10. Pruebas y QA

| Área | Caso | Criterio |
| --- | --- | --- |
| Conexión | Servidor apagado | Banner claro en < 5 s, sin cuelgue |
| Conexión | Credenciales malas | Estado `AUTH_FAILED`, no reintento infinito |
| Conexión | Reinicio del servidor | La app se recupera sin reiniciar |
| Schema | Cliente viejo vs base nueva | Bloqueo seguro + mensaje |
| Schema | Base desactualizada | Bloqueo hasta correr el runner |
| Migraciones | 3 clientes a la vez | Ninguno migra; solo el runner |
| Eventos | 2 PCs, misma muestra | Evento cruzado < 1 s, sin payload erróneo |
| Citas | 2 PCs, mismo profesional/hora | Solo una cita se crea |
| Estancias | 2 PCs, última cama libre | Solo un ingreso |
| Facturas | Fallo a mitad de ítems | Rollback total, sin factura huérfana |
| Stock | 2 salidas simultáneas | Nunca stock negativo |
| Archivos | Adjunto en PC 1, abrir en PC 2 | Se ve y se descarga |
| Backup | `gbak` + restore | Conteos idénticos, `gfix` limpio |
| Carga | 3 PCs, 15 min de uso | Sin deadlocks ni timeouts del pool |

Automatizable en CI: migraciones concurrentes, CAS de estados, grants, health
tipado. Manual: red, firewall, eventos y archivos.

---

## 11. Commits sugeridos

1. `feat(db): connect to Firebird 5 server over TCP` (Fase 3)
2. `feat(db): run schema migrations from an admin runner` (Fase 4)
3. `feat(db): remote events and concurrent-safe operations` (Fase 5)
4. `feat(files): share attachments and server-side backups` (Fase 6)
5. `build: ship Firebird client runtime and server docs` (Fase 7)
6. `docs(ops): firebird server cutover runbook` (Fase 8)

Cada commit con: `cargo test`, `npm test`, `tsc --noEmit`, `npm run lint`,
`npm run build` en verde.

---

## 12. Matriz de riesgos

| Riesgo | Prob. | Impacto | Mitigación |
| --- | --- | --- | --- |
| Compartir el `.fdb` por SMB | Media | Crítico | Servidor Firebird, nunca SMB |
| 3 clientes migrando a la vez | Alta | Alto | Runner único + verificación de versión |
| Servidor en un PC de trabajo | Media | Alto | Mini-PC dedicado / UPS / servicio auto |
| Firewall bloquea 3050/3051 | Media | Medio | Reglas explícitas + script de verificación |
| Eventos no llegan | Media | Medio | `RemoteAuxPort` fijo + fallback a invalidación genérica |
| Pool sin reconexión | Alta | Medio | Condvar + timeout + descarte de conexiones muertas |
| Archivos en rutas locales | Alta | Alto | BLOB en Firebird: el archivo viaja con la fila |
| Crecimiento de la BD por BLOB | Media | Medio | Límite por adjunto, `gbak`/`nbackup`, retención y limpieza de huérfanos |
| DPAPI no comparte secretos | Alta | Medio | Clave maestra de clínica + AEAD; clave envuelta con DPAPI por PC |
| Backups inexistentes | Media | Crítico | `gbak` diario + restore probado |
| Actualización descoordinada | Media | Alto | Runner + rango de schema soportado |
| Pérdida de datos en el corte | Baja | Crítico | Backup + restore verificado + rollback |

---

## 13. Decisiones (actualizado 2026-11-18)

### Resueltas

| # | Decisión | Respuesta |
| --- | --- | --- |
| 1 | Hardware del servidor | **Tiny nuevo compatible con Windows 11** (8.ª gen+/N100, 16 GB, SSD, Ethernet); se descarta el de 7.ª gen con Windows 10 |
| 2 | Datos iniciales | **Base limpia**: el negocio inicia desde cero |
| 3 | Bases previas | No existen; no hay nada que fusionar |
| 4 | Adjuntos/logos/certificados | **BLOB en Firebird** (sin SMB, por los problemas de red con Windows 11) |
| 5 | Secretos de IA/SMTP | **Compartidos**, cifrados con clave maestra de clínica envuelta con DPAPI por PC |

### Pendientes de definir con el cliente

1. **Red:** ¿cableada o Wi-Fi? (Cableada recomendada.)
2. **IP fija o DNS local.** (IP fija + alias recomendado.)
3. **Nombre del servidor** (`ISALAB-SRV`).
4. **Quién ejecuta el runner** de migraciones y quién administra el servidor.
5. **Política de backups:** frecuencia, retención, copia externa y responsable.
6. **Ventana de corte** y tolerancia a downtime.
7. **Credenciales del personal** que se crearán en la base limpia.

---

## 14. Anexo — Comandos de referencia

```bat
:: Servicio (Windows)
cd /d "C:\Firebird\Firebird_5_0"
install_service.bat
instsvc query

:: Conexión remota (cliente)
isql -user ISALAB_APP -password <clave> 192.168.1.10/3050:ISALAB

:: ODS
SQL> SELECT RDB$ODS_VERSION FROM RDB$DATABASE;

:: Backup / restore
gbak -b -t -v -user SYSDBA -password <clave> ^
     localhost/3050:ISALAB C:\Firebird\backups\isalab.fbk
gbak -c -r -v C:\Firebird\backups\isalab.fbk ^
     C:\Firebird\data\isalab.fdb

:: Validación
gfix -validate C:\Firebird\data\isalab.fdb
gfix -sweep    C:\Firebird\data\isalab.fdb

:: Firewall (PowerShell, ejecutar como admin)
New-NetFirewallRule -DisplayName "Firebird ISALAB" -Direction Inbound ^
  -Protocol TCP -LocalPort 3050,3051 -Action Allow ^
  -RemoteAddress 192.168.1.21,192.168.1.22,192.168.1.23 -Profile Private
```

```ini
# C:\Firebird\Firebird_5_0\databases.conf (servidor)
ISALAB = C:\Firebird\data\isalab.fdb
security5 = C:\Firebird\Firebird_5_0\security5.fdb
```

```ini
# firebird.conf (cliente ISALAB)
Providers = Remote
RemoteServicePort = 3050
AuthClient = Srp256, Srp, Win_Sspi, Legacy_Auth
```

---

## 15. Anexo — Windows 11 y red (por qué se evita SMB)

**Firebird no usa SMB ni el descubrimiento de red de Windows.** La app abre un
socket TCP directo al puerto 3050 del servidor. Aunque el servidor no aparezca
en el Explorador de Windows, la base funciona si `Test-NetConnection` al 3050
da `TcpTestSucceeded: True`:

```powershell
Test-NetConnection -ComputerName 192.168.1.10 -Port 3050
Test-NetConnection -ComputerName 192.168.1.10 -Port 3051
```

Los problemas que describiste ("creo el equipo central y no puedo acceder a sus
recursos") son de **SMB / compartición de archivos**, y en Windows 11 suelen
venir de una de estas causas:

| Causa | Síntoma | Solución |
| --- | --- | --- |
| Perfil de red en **Público** | El firewall bloquea archivos/impresión y descubrimiento | Poner la red en **Privada** (Configuración → Red → Tipo de perfil), o `Set-NetConnectionProfile -NetworkCategory Private` |
| Descubrimiento y compartición desactivados | El PC no aparece en "Red" | Panel de control → Centro de redes → Configuración de uso compartido avanzado → activar descubrimiento y compartición (perfil Privado) |
| Firewall sin reglas de SMB | `\\IP\share` no abre; 445 cerrado | Habilitar el grupo "Compartir archivos e impresoras"; abrir TCP 445 solo en la red privada |
| **Acceso invitado deshabilitado** (desde Win10 1709) | Pide credenciales y falla con "acceso denegado" | Crear una cuenta **local** con contraseña en el servidor y usarla; no intentar "guest" |
| Cuenta Microsoft | No acepta usuario/contraseña de la cuenta MS | Crear usuario local `isalabfiles` en el servidor y darle permisos al share |
| **SMB signing obligatorio** (Windows 11 24H2) | Falla contra NAS/Samba antiguos | Actualizar Samba/NAS o habilitar firma; en el cliente: `Get-SmbClientConfiguration` para diagnosticar |
| Nombre de equipo no resuelto | `\\ISALAB-SRV` falla, `\\192.168.1.10` funciona | Usar IP fija; si se quiere nombre, agregarlo a `hosts` o al DNS local |
| Aislamiento de clientes en Wi-Fi | Los PCs no se ven entre sí | Red cableada o desactivar "AP/client isolation" |
| Antivirus/firewall de terceros | Bloquea 445/3050 | Exclusión o regla explícita |

**Decisión del plan:** para no depender de nada de lo anterior, los adjuntos se
guardan como **BLOB** y el único requisito de red es TCP **3050/3051**. SMB
queda solo como alternativa si en el futuro se prefiere un share:

```powershell
# Solo si se opta por SMB (no es la ruta recomendada)
New-SmbShare -Name "isalab-files" -Path "C:\Firebird\files" `
  -FullAccess "ISALAB-SRV\isalabfiles"
net use \\192.168.1.10\isalab-files /user:ISALAB-SRV\isalabfiles <clave> /persistent:yes
```
