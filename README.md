# ISALAB · Laboratorio Veterinario

Sistema de escritorio moderno para **laboratorios veterinarios** (inspirado en
[ISALAB](https://github.com/CORJAR-Computers/ISALAB.git)): historiales clínicos
multiespecie, muestras analíticas con trazabilidad, vacunación, reportes PDF
firmados y facturación.

## Stack

| Capa | Tecnología |
| --- | --- |
| Shell | Tauri v2 (Rust) |
| Base de datos | Firebird 5.0 **Embedded** (`rsfbclient` 0.27, `dynamic_loading`) |
| Frontend | React 19 + TypeScript + Vite |
| Estilos | Tailwind CSS v4 + shadcn/ui |
| Estado | Zustand + TanStack Query |
| IPC type-safe | specta / tauri-specta (`src/bindings.ts` auto-generado) |
| PDF | `printpdf` (server-side, hito 2) |

## Requisitos

- Rust **stable** (≥ 1.77) con target `x86_64-pc-windows-msvc` + MSVC Build Tools
- Node.js ≥ 20 y npm
- **Firebird 5 Embedded** (solo desarrollo): copia `fbclient.dll` a
  `src-tauri/binaries/firebird/` (ver [README de binarios](src-tauri/binaries/firebird/README.md)).
  El **instalador de producción v0.3.0 ya incluye el motor embebido**.

## Primer arranque

```bash
npm install            # dependencias frontend
npm run icons          # genera src-tauri/icons (PNG + ICO)
npm run tauri:dev      # compila Rust + arranca la app con la UI
```

En el primer arranque válido la app:

1. Crea `isalab.fdb` en `app_data` (si no existe) con página 16 KB.
2. Aplica las migraciones versionadas (`src-tauri/migrations/*.sql`).
3. Siembra el catálogo (especies, razas, analitos, rangos de referencia por
   especie/edad/sexo, configuración de clínica).
4. Suscribe listeners a los **eventos nativos de Firebird** (`POST_EVENT`) y
   re-emite cambios de muestras/resultados al frontend en tiempo real.
5. Siembra la contraseña del usuario `admin` (ver [Autenticación y Seguridad](#autenticación-y-seguridad)).

> La base de datos nunca se commitea (`.gitignore`). Cada instalación la crea
> localmente.

## Instalación en producción (v0.3.0)

Desde la **v0.3.0** la aplicación se distribuye como **instalador NSIS**
(`ISALAB_0.3.0_x64-setup.exe`) con el **motor Firebird 5 Embedded incluido**:
no requiere instalar Firebird ni configurar nada en la máquina destino.

1. Descarga la última versión desde **Releases**:
   <https://github.com/CORJAR-Computers/ISALAB-TR/releases>
2. Ejecuta el instalador (instala por usuario, sin privilegios de
   administrador) y abre la aplicación.
3. En el primer arranque la app crea su base de datos `isalab.fdb` en la
   carpeta de datos de la app y aplica migraciones + seed automáticamente.

El pipeline de release (`release.yml`) compila el instalador en GitHub
Actions, adjunta el artefacto al release y genera notas de cambios
automáticas.

### Auto-actualización (v0.3.2)

Desde la **v0.3.2** la app incluye el **plugin oficial de auto-actualización**
de Tauri. Al iniciar (solo en builds de producción) comprueba si hay una
versión nueva en el release más reciente de GitHub; si la hay, muestra un
diálogo para descargarla, instalarla y reiniciar la aplicación.

- **Endpoint**: `…/releases/latest/download/latest.json` (generado por
  `tauri build` junto al instalador gracias a `createUpdaterArtifacts: true`).
- **Firma**: cada instalador se firma con la **clave privada de minisign**
  (par `isalab.key` / `isalab.key.pub`); el cliente verifica la firma con la
  clave pública embebida en `tauri.conf.json`. Actualizaciones no firmadas
  por el mantenedor **no se instalan**.
- **Instalación silenciosa**: NSIS en modo `currentUser` + updater en modo
  `passive` (barra de progreso, sin prompts de elevación).

#### Claves de firma y CI

La clave privada se guarda como secreto del repositorio (necesaria para que
`tauri build` genere los artefactos del updater):

| Secreto | Valor |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | contenido de `isalab.key` (cifrado) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | frase de paso (vacía si se generó con `-p ""`) |

Regeneración (solo si se pierde la clave; invalidaría las instalaciones
anteriores):

```bash
npx tauri signer generate -w ~/.tauri/isalab.key --ci -p ""
# copia isalab.key a los secretos y isalab.key.pub a tauri.conf.json > plugins.updater.pubkey
```

### Compilación local del instalador

```bash
npm run tauri:build   # build release + instalador NSIS
```

El instalador queda en `target/release/bundle/nsis/` dentro del `target-dir`
configurado (ver sección siguiente) e incluye el frontend compilado y
`binaries/firebird/**` (fbclient.dll + ICU + plugins).

## Compilación en Windows: target fuera del antivirus

Windows Defender (y otros antivirus) escanean en tiempo real el directorio
`target/` mientras `rustc` arma los `.rlib`. Si el archivo está bloqueado en
ese instante, la compilación falla con `os error 32` ("el archivo está siendo
utilizado por otro proceso") y quedan residuos `.temp-archive`. Para evitarlo
en este proyecto el target se mueve a una ubicación estable **fuera del árbol**
y se excluye del antivirus.

### 1. Directorio de compilación (`CARGO_TARGET_DIR`)

`npm run setup:target` detecta el mejor disco disponible, crea la carpeta y
escribe `target-dir` en `src-tauri/.cargo/config.toml` (sin editarlo a mano):

```bash
npm run setup:target             # configura y crea la carpeta
npm run setup:target -- --print  # solo muestra la ruta que elegiría
npm run setup:target -- --unset  # revierte: cargo vuelve a src-tauri/target
npm run setup:target -- --move   # mueve src-tauri/target (caché) a la nueva ruta
```

Resultado esperado (Windows): `[build] target-dir = "D:/rust-targets/isalab"`;
en Unix, `~/.rust-targets/isalab`. Prioridad de resolución: variable de entorno
`CARGO_TARGET_DIR` > `target-dir` del config > `src-tauri/target` por defecto.

> [!NOTE]
> `src-tauri/.cargo/` está en `.gitignore`: la ruta absoluta es específica de
> cada máquina y **no debe commitearse** (rompería el CI, que cachea
> `src-tauri/target`).

### 2. Exclusión de Windows Defender

Además de mover el target, hay que excluir la nueva ruta del antivirus. El
script puede aplicarla automáticamente (aparecerá un aviso UAC para elevar
permisos):

```bash
npm run setup:target -- --defender-exclusion
```

También puedes aplicarla a mano (PowerShell como administrador):

```powershell
# Directorio de compilación (target de cargo)
Add-MpPreference -ExclusionPath 'D:\rust-targets'

# fbclient.dll de Firebird 5 Embedded (runtime de la app)
Add-MpPreference -ExclusionPath 'D:\Proyectos\ISALAB-TR\src-tauri\binaries\firebird'
```

La exclusión de `binaries/firebird` evita que el antivirus bloquee `fbclient.dll`
al cargar Firebird 5 Embedded cuando se ejecuta la app.

### 3. Limpieza automática de residuos (`.temp-archive`)

`scripts/clean-target-orphans.mjs` elimina los directorios `*.temp-archive`
huérfanos que deja una compilación fallida, resolviendo el target efectivo con
`cargo metadata` (sigue la configuración de la sección 1). Se ejecuta
automáticamente antes de cada compilación:

```bash
npm run clean:target    # manual, si hiciera falta
npm run tauri:dev       # ya incluye la limpieza previa
npm run tauri:build     # ídem
```

## Autenticación y Seguridad

> [!CAUTION]
> **IMPORTANTE**: Al primer arranque el usuario **`admin`** queda habilitado con la contraseña por defecto **`admin123`** (Argon2id). Por razones de seguridad en entorno clínico, **debes cambiar esta contraseña inmediatamente** en tu primer inicio de sesión desde el módulo de Usuarios o Configuración.

- La sesión es local y única (una ventana, un operador a la vez).
- Control de acceso basado en roles (RBAC: `ADMIN`, `VETERINARIO`, `AUXILIAR`) enforced en los comandos nativos de Rust.
- **RBAC completo (v0.3.0)**: los **35 comandos** Tauri requieren sesión activa (`require_session`); las mutaciones críticas (usuarios, configuración, auditoría, logos, certificado PKCS#12, copias de seguridad) requieren rol `ADMIN` (`require_admin`).
- **Rate limiting de login (v0.3.0)**: 5 intentos fallidos bloquean el usuario durante 5 minutos.
- **Cifrado de secretos (DPAPI de Windows)**: la clave de IA (Groq) se almacena **cifrada con DPAPI** en `CLINIC_SETTINGS` (formato `enc:v1:<base64>`), ligada al usuario de Windows que la configuró. Una copia de la base de datos no expone la clave; los valores legacy en texto plano se re-cifran automáticamente al primer acceso.
- **Auditoría**: tabla `USER_AUDIT_LOG` registra inicios/cierres de sesión, intentos fallidos de login, cambios de contraseña, creación de usuarios, cambios de configuración, importación de logos/certificados y transiciones de estado en muestras, facturas, consultas y cirugías. Desde la v0.3.0 el historial se consulta en la **UI de Auditoría** (solo `ADMIN`, tabla paginada con filtros).
- **CSP**: Content Security Policy configurado (`script-src 'self'`, sin eval).
- **Tests**: 208 tests de Rust (`cargo test`) + 102 de frontend con Vitest (`npm test`).

## Firmado de código (SmartScreen)

El pipeline de release (`release.yml`) incluye el **firmado Authenticode con
SignPath Foundation** (gratis para proyectos OSS) para eliminar el aviso
"Editor desconocido" de SmartScreen en el instalador NSIS.

> [!NOTE]
> Los pasos de firmado están **desactivados hasta que se configuren los
> secretos de SignPath** (`if: env.SIGNPATH_API_TOKEN != ''`). Mientras
> tanto el release sale sin firmar: Windows muestra el aviso SmartScreen (no
> bloquea la instalación).

### Runner del release (automático)

El job `release` **elige su runner automáticamente** mediante el job
`decide-runner` (`release.yml`), según exista o no el secreto:

| `SIGNPATH_API_TOKEN` | Runner del job `release` |
| --- | --- |
| **Definido** (SignPath activo) | `windows-latest` (**GitHub-hosted**) |
| **Ausente** (hoy) | self-hosted (`isalab-release`) |

Esto importa para el **programa OSS de SignPath**: exige que **todos** los
jobs del workflow que llevan a la solicitud de firma corran en runners
GitHub-hosted. Al configurar `SIGNPATH_API_TOKEN`, el workflow pasa solo a
`windows-latest` y cumple el requisito sin tocar nada más; sin él, seguimos
en el runner self-hosted (evita los fallos transitorios de capacidad de
`windows-latest`) y los pasos de firma se saltan igualmente.

### Cómo activar el firmado

> [!TIP]
> Para los mantenedores: abre un issue con la plantilla
> **"🔏 Solicitud OSS — Firma de código (SignPath Foundation)"**
> (`.github/ISSUE_TEMPLATE/signpath-oss-request.yml`): documenta el proceso
> completo con checklists y sirve de tracker hasta el primer release firmado.

1. **Solicita el acceso OSS** en <https://signpath.io> (SignPath Foundation,
   gratis): el repo debe ser público (sí), usar licencia OSI aprobada
   (AGPL-3.0) y runners de GitHub-hosted — el workflow ya los usa
   automáticamente cuando SignPath está activo (ver *Runner del release*
   arriba). El proyecto debe ser aceptado por la fundación (colas de
   espera).
2. **Instala el GitHub App de SignPath** y vincúlalo al repositorio.
3. **Crea el Proyecto, la Artifact Configuration y la Signing Policy** en la
   consola de SignPath. La Artifact Configuration debe tener raíz
   `<zip-file>` (el instalador se sube como artefacto ZIP de GitHub Actions).
4. **Añade los 4 secretos juntos** al repositorio (el workflow se activa solo
   cuando `SIGNPATH_API_TOKEN` está definido; si falta alguno de los otros,
   el paso de firma fallará):

| Secreto | Valor |
| --- | --- |
| `SIGNPATH_API_TOKEN` | API token con permisos de submitter en el proyecto/policy |
| `SIGNPATH_ORG_ID` | ID de la organización en SignPath |
| `SIGNPATH_PROJECT_SLUG` | Slug del proyecto |
| `SIGNPATH_SIGNING_POLICY_SLUG` | Slug de la signing policy (p. ej. `release-signing`) |

   Si la **Artifact Configuration** de SignPath define parámetros de usuario
   (por ejemplo `version`, que SignPath suele exigir por su validación de
   metadatos estrictos), añádelos al paso `Submit signing request` del
   workflow:

   ```yaml
   parameters: |
     version: ${{ toJSON(github.ref_name) }}
   ```

5. **Cada release requiere aprobación manual** en el portal de SignPath (un
   "approver" del proyecto) antes de firmarse; el workflow espera hasta 1 h.

### Cómo funciona en el pipeline

1. `tauri build` genera el instalador, su `.sig` de minisign y el manifiesto
   `latest.json` del updater.
2. El instalador sin firmar se sube a SignPath (`upload-artifact` +
   `github-action-submit-signing-request@v2`).
3. Al volver firmado, se sustituye el `.exe` y se **vuelve a firmar con
   minisign** (`tauri signer sign`): la firma Authenticode modifica el binario
   e invalidaría la firma del auto-updater, que verifica cada instalador con
   la clave pública embebida.
4. `latest.json` se regenera con la firma nueva y todo se adjunta al release.

> **Sigstore/cosign no sirve para esto**: firma artefactos OCI/blobs y
> certificados EFI, pero **no genera firmas Authenticode para .exe** de
> Windows, por lo que no elimina SmartScreen.

Alternativas si SignPath no encaja:

| Opción | Costo | Notas |
| --- | --- | --- |
| **Azure Trusted Signing** | ~9 USD/mes + certificado | Configurable en CI con `azure/artifact-signing-action@v2`; requiere suscripción Azure y validación de identidad |
| Certificado EV propio | ~300–500 USD/año | Firma local con `signtool`; requiere mantener el certificado y el HSM |

## Estructura

```text
src/                     # Frontend React
├── bindings.ts          # Tipos TS generados por specta (se regeneran en build)
├── lib/api.ts           # Wrappers tipados de los comandos (invoke)
├── stores/ui-store.ts   # Zustand: tema, sidebar, navegación
├── hooks/               # TanStack Query + listener de eventos Firebird
├── components/ui/       # shadcn/ui
└── features/            # patients/, clinical-history/, placeholder/

src-tauri/               # Backend Rust
├── migrations/          # SQL versionado (0001 schema, 0002 seed)
├── binaries/firebird/   # fbclient.dll embebida (se bundlea)
└── src/
    ├── db/              # init Embedded, pool, migraciones (SET TERM), eventos
    ├── models/          # DTOs con derive specta::Type
    ├── repositories/    # Patrón Repository (pacientes, historial, muestras, settings)
    ├── commands/        # Comandos Tauri tipados
    └── pdf_templates/   # Generador de reportes (printpdf 0.12, ops-based)
```

## Dominio clínico

- **Multiespecie**: canino, felino, equino, bovino, ovino, caprino, porcino,
  aves y exóticos (tabla `SPECIES` → `BREEDS`).
- **Trazabilidad de muestras**: código único `M-YYYY-NNNN`, estados
  `RECIBIDA → EN_PROCESO → FINALIZADA → ANULADA`, cadena de custodia y
  vinculación inequívoca al paciente (`SAMPLES` + `EVENT_LOG`).
- **Valores de referencia por especie + edad + sexo**: `REFERENCE_RANGES` con
  tramos de edad en meses; el SP `SP_VALIDATE_ANALYTICAL_RESULT` calcula el
  estado clínico (`BAJO/NORMAL/ALTO/SIN_RANGO`) de cada resultado al registrarse.
- **Colombia**: cédula/NIT, IVA 19% configurable, tarjeta profesional MVZ en
  los reportes, moneda COP.

## Roadmap

- [x] **Hito 1**: scaffolding, schema Firebird 5, CRUD pacientes + historial
  clínico, tipos TS vía specta.
- [x] **Hito 2**: reportes PDF con `printpdf` 0.12 (firma gráfica y **firma
  digital PKCS#12** .p12/.pfx), laboratorio (mesa de trabajo, resultados por
  analito con validación de rangos), configuración de la clínica y
  autenticación local (Argon2id, roles ADMIN/VETERINARIO/AUXILIAR).
- [x] **Hito 3**: agenda de consultas y cirugías (tabla `SURGERIES` con
  anestesia y estados), vacunación y desparasitación desde el historial y
  panel de control con métricas y próximas citas.
- [x] **Hito 4**: facturación (facturas con IVA 19% configurable, métodos de
  pago y estados EMITIDA/PAGADA/ANULADA) y gestión de usuarios (roles, cambio
  de contraseña, `MUST_CHANGE_PASSWORD`).
- [x] **Reportes PDF completos**: además del informe de laboratorio, se
  generan con `printpdf` la fórmula médica (receta), el consentimiento
  informado de cirugía, el recibo/comprobante de pago de factura, el
  certificado quirúrgico y el carnet de vacunación (selección por pestañas
  en Reportes PDF → Generar).
- [x] **v0.3.0 — Endurecimiento y producción**: RBAC en los 35 comandos,
  rate limiting de login, auditoría ampliada con UI de consulta (ADMIN),
  logos secundarios y preferencia por paciente, refactor de plantillas PDF
  (`layout.rs`), instalador NSIS con Firebird embebido y pipeline de release
  en GitHub Actions.
