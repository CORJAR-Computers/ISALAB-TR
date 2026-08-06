# Changelog

All notable changes to ISALAB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **Etiquetas de tubos de muestras** (`generate_sample_labels`): hoja Carta
  con grilla 2×4 de etiquetas adhesivas, cada una con el código de
  trazabilidad en grande + código de barras Code 128 (escaneable), datos del
  paciente, tipo de muestra, fecha de recepción y responsable. Botón
  individual en el detalle de muestra y selección múltiple por checkboxes en
  la mesa de trabajo (máx. 100 por hoja). Reutiliza printpdf/barcoders.
- **Exportación CSV de muestras y resultados** (`export_samples_csv`,
  `export_results_csv`): guarda a disco con diálogo nativo, separador `;` y
  BOM UTF-8 para apertura directa en Excel hispano. Filtros de la mesa de
  trabajo (estado/búsqueda) aplican a la exportación.
- **Métricas de laboratorio en el panel de control**: tiempo promedio de
  procesamiento (recepción→finalización en horas), % de muestras finalizadas
  con valores fuera de rango, tendencia de volumen diario de los últimos 7
  días y ranking de los 5 analitos más solicitados.

## [0.3.3] - 2026-08-06

### Changed

- **Migración a Zod 4** (`zod 3→4.4` + `@hookform/resolvers 5.0→5.7`):
  los schemas con `z.coerce` exigen el patrón de tres genéricos en
  `useForm<z.input, unknown, z.output>`, se renombró
  `invalid_type_error → error` y `z.string().email() → z.email()`. Incluido
  en el PR #1 de Dependabot junto con `lucide-react 0.525→1.28`.
- **Toolchain de build modernizada**: `vite 7→8` (minifier Oxc en lugar de
  esbuild), `vitest 3→4`, `@vitejs/plugin-react 4→6`, `jsdom 26→30`,
  `@types/node 22→26`, `@vitest/coverage-v8 3→4` y
  `@testing-library/jest-dom 6→7`. TypeScript se mantiene en 5.8 a
  propósito: `typescript-eslint` aún no soporta TS 7 (rewrite en Go);
  llegará en su propio PR individual.
- **Dependabot**: los grupos ahora agrupan solo `minor`/`patch`; los majors
  llegan en PRs individuales para no bloquear las actualizaciones seguras.

### Added

- **Escáner de paciente por código** (`PatientScanner.tsx`): input con
  autofocus en Pacientes que resuelve el código PAC-… al pulsar Enter
  (envío del escáner) mediante el nuevo hook `usePatientByCode` y el comando
  `get_patient_by_code` expuesto en `api.ts`. Si el paciente existe abre su
  ficha (Historial Clínico); si no, muestra «no encontrado» con el código
  consultado. Incluye tests de `renderHook` (`usePatientByCode.test.tsx`:
  trim, query deshabilitada con código vacío y datos del paciente).
- **Smoke test E2E con Playwright** (`e2e/` + `playwright.config.ts`):
  `npm run test:e2e` valida login → dashboard → flujo completo de una
  muestra (RECIBIDA → EN PROCESO → resultado → FINALIZADA) contra la UI
  real en el dev server de Vite con el IPC de Tauri mockeado
  (`e2e/ipc-mock.script.js`). Nuevo job `e2e` en `ci.yml` con
  `playwright install --with-deps chromium`; el backend real sigue cubierto
  por `cargo test`.
- **Código de barras Code 128 en el carnet de vacunación** (`vaccines.rs` +
  `layout.rs`): el código del paciente (PAC-…) se imprime como barras
  vectoriales (crate `barcoders` 2.0.0, sin incrustar fuentes) y es
  escaneable desde el escáner de Pacientes para abrir la ficha al instante.
- **Firmado Authenticode con SignPath Foundation en el pipeline de release**
  (`release.yml`): elimina el aviso SmartScreen del instalador NSIS. Los
  pasos de firmado son condicionales a los secretos `SIGNPATH_*` (si no
  están configurados, el release sale sin firmar como hasta ahora). Tras
  recibir el instalador firmado se vuelve a firmar con minisign
  (`tauri signer sign`) y se regenera `latest.json`, porque la firma
  Authenticode modifica el `.exe` e invalidaría la firma del auto-updater.

## [0.3.2] - 2026-08-05

### Added

- **Auto-actualización (plugin oficial de Tauri Updater)**: al iniciar la app
  (solo builds de producción) se comprueba si hay una versión nueva en GitHub
  Releases; si existe, un diálogo permite descargarla, instalarla y reiniciar
  (`use-app-updater.ts` + `UpdateDialog.tsx` con barra de progreso).
  - `createUpdaterArtifacts: true` → `tauri build` genera el instalador, su
    firma `.sig` y el manifiesto `latest.json` (endpoint
    `…/releases/latest/download/latest.json`).
  - Instalación silenciosa: NSIS `currentUser` + updater `passive`.
  - Clave de firma minisign (`isalab.key`) guardada como secreto
    `TAURI_SIGNING_PRIVATE_KEY` en GitHub; la clave pública va embebida en
    `tauri.conf.json` y el cliente verifica cada actualización.
  - `release.yml` ahora adjunta `bundle/**/*` (incluye `.sig` y
    `latest.json`); `ci.yml` inyecta la clave de firma en el job `tauri-build`.

### Security

- **Auditoría de dependencias Rust** (`cargo audit` 0.22.2): 0 vulnerabilidades
  explotables. `lopdf` 0.36 (alto 7.5, vía `pdf_signer` para firmas PKCS#12)
  y `rsa` 0.9.10 (medio, Marvin Attack) no tienen fix disponible; el riesgo
  real es bajo: la app solo firma PDFs generados por ella misma y nunca
  parsea PDFs de terceros. `pdf_signer` (GPL-3.0) es compatible con la
  licencia AGPL-3.0 del proyecto (sección 13).
- **`cargo audit` en CI**: el job `backend` de `ci.yml` ejecuta la auditoría
  de dependencias en cada push/PR. Los dos advisories conocidos sin fix
  (`RUSTSEC-2026-0187` lopdf y `RUSTSEC-2023-0071` rsa) se ignoran de forma
  explícita y documentada, de modo que el pipeline falla solo si aparece una
  vulnerabilidad nueva en el árbol de dependencias.

## [0.3.1] - 2026-08-05

### Security & Hardening

- **Secretos cifrados en la BD local (DPAPI de Windows)**: la clave de IA
  (Groq) ya no se almacena en texto plano. Nuevo módulo `crypto.rs` la cifra
  con `CryptProtectData` (ámbito del usuario de Windows) y la persiste como
  `enc:v1:<base64>` en `CLINIC_SETTINGS`. Los valores legacy en texto plano se
  re-cifran automáticamente en el primer acceso; si la BD se mueve a otro
  usuario o máquina, la clave no se expone (se pide de nuevo en Ajustes). En
  builds de desarrollo no-Windows el módulo es un passthrough documentado (el
  producto de producción es Windows).

### Added

- Tests de cifrado en `crypto.rs` (roundtrip, legacy, corrupto) y de
  repositorio (`test_groq_api_key_is_encrypted_at_rest`,
  `test_legacy_plaintext_key_is_migrated_on_read`). Total de tests Rust:
  202 → **208**.

### Build & CI

- **Instalador más pequeño**: nuevo `scripts/prune-firebird.mjs` reduce el
  motor Firebird al subconjunto embedded (fbclient + ICU + plugins + intl +
  tzdata + SECURITY5.FDB), eliminando las herramientas de servidor
  (firebird.exe, gbak, gfix…), docs y el instalador de runtime (~68 MB →
  ~49 MB). Se ejecuta en el pipeline de CI (release + tauri-build) y en
  `npm run tauri:build`; el instalador NSIS baja de ~20 MB a ~16 MB.
- **Actions de GitHub a Node 24**: checkout v7, setup-node v7, cache v6,
  upload-artifact v7 y action-gh-release v3 (runtime `node24`, sin aviso de
  deprecación) y `node-version: 24` en todos los jobs.

## [0.3.0] - 2026-08-05

### 🎯 Release Summary

Production-hardening release: full RBAC enforcement across every command,
login rate limiting, an expanded audit trail with an admin UI, secondary
clinic logos for PDF reports, a shared PDF layout module, and a CI pipeline
that produces installers with the Firebird embedded engine bundled.

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Rust tests | 30 | 202 | **+573%** |
| TypeScript tests | 102 | 102 | - |
| Clippy warnings | 28 | 0 | **-100%** |
| Repositories tested | 2 | 11 | **+450%** |
| AI features | Basic | Enhanced + Cached | ✅ |
| Commands with RBAC | — | 35/35 | ✅ |

### Security & Hardening

- **RBAC completo**: `require_session()` aplicado a los 35 comandos Tauri; `require_admin()` para acciones sensibles (usuarios, configuración, auditoría, logos, certificado PKCS#12, copias de seguridad)
- **Guard consolidado**: se eliminó el `current_user()` duplicado; todo pasa por `auth::require_session()`
- **Rate limiting de login**: 5 intentos fallidos → bloqueo de 5 minutos por usuario (`LoginAttempts` en `AppState`)

### Audit Log

- Nuevo modelo `AuditLogEntry` y repositorio con paginación `FIRST ? SKIP ?`
- Comando `list_audit_log` protegido con `require_admin()`
- UI **AuditLogPage**: tabla paginada (50/página), badges semánticos por acción y vista exclusiva de administradores
- Eventos registrados: `LOGIN`, `LOGIN_FAILED` (usuario inexistente, inactivo o contraseña incorrecta), `USER_CREATED`, `PASSWORD_CHANGED`, `SETTINGS_CHANGED`, `LOGO_IMPORTED`, `SECONDARY_LOGO_*`, `CERTIFICATE_IMPORTED`, `SAMPLE_STATUS_CHANGE`, `INVOICE_STATUS_CHANGE`, `CONSULTATION_STATUS_CHANGE`, `SURGERY_STATUS_CHANGE`

### Features

- **Logos secundarios** (migraciones `0007_secondary_logos` + `0008_patient_preferred_logo`): importación, listado y borrado de logos adicionales para los reportes PDF; **logo preferido por paciente**; selector de logo en el diálogo de generación de reportes con opción de guardar la preferencia

### PDF Refactor

- Nuevo `pdf_templates/layout.rs` con helpers compartidos: `section_title`, `draw_grid`, `draw_patient_block`, `draw_results`, `draw_signature`, `draw_lab_note`, `draw_footer` y `ReportSignature`
- Plantillas `clinical`, `surgical`, `financial` y `vaccines` reescritas sobre `layout.rs` (menos duplicación, firma consistente)

### Production Build & CI

- **Instalador con Firebird embebido**: los jobs `release` y `tauri-build` de GitHub Actions ahora descargan el runtime Firebird 5 y lo colocan en `src-tauri/binaries/firebird/` para que el glob de recursos lo empaquete (el motor está gitignored, así que sin este paso el instalador salía sin base de datos)
- **Code-splitting del frontend**: bundle principal de 1266 kB → 207 kB en chunks cacheables (react, charts, markdown, icons, vendor); resuelto el ciclo `vendor ↔ react-vendor` causado por `@floating-ui/react`
- **Lint y clippy limpios**: eliminados los últimos `any` del diálogo de reportes (tipado estricto), `ptr_arg` y `unnecessary_cast`; build release sin warnings (import de `specta` gated por `debug_assertions`)

### Fixed

- **Rust clippy warnings (28 errors → 0)**: Resolved all `cargo clippy -- -D warnings` errors across the backend
  - `clippy::type-complexity` (21): Added type aliases for complex tuple types in repositories (`PatientRow`, `SampleRow`, `LabResultRow`, `SurgeryRow`, `VaccineRow`, `InvoiceRow`, `InvoiceListItemRow`, `SampleListItemRow`, `VaccineListItemRow`, `ConsultationRow`, `ConsultationListItemRow`, `UserRow`, `AuditLogRow`, `OwnerRow`, `TrendPointRow`, `SpeciesRow`, `BreedRow`, `AnalyteRow`)
  - `clippy::needless-borrow` (3): Removed unnecessary `&` references in `clinical_history.rs`, `invoices.rs`, `vaccines.rs`
  - `clippy::useless-format` (1): Replaced `format!()` with string literal in `samples.rs`
  - `clippy::useless-vec` (2): Replaced `vec![]` with array literals in `layout.rs`
  - `clippy::new-without-default` (1): Added `impl Default for PdfBuilder` in `builder.rs`

- **Unsafe `unwrap()` in production code**: Replaced `unwrap()` with proper error handling in `commands/db.rs` (`create_local_backup`)

- **Settings repository bug fix**: Fixed `get()` function to return `None` for empty optional fields instead of `Some("")`. Previously, fields like `address`, `phone`, `city`, `logo_path`, `vet_license`, and `groq_api_key` would return `Some("")` when not set, causing incorrect behavior in the frontend.

### Added

#### Integration Tests (89 new tests)

| Repository | Tests | Coverage |
|------------|-------|----------|
| `patient.rs` | 7 | CRUD, search, owner relations |
| `samples.rs` | 5 | Status transitions, filters |
| `vaccines.rs` | 3 | Create, by_patient, list |
| `surgeries.rs` | 5 | Status transitions, filters |
| `invoices.rs` | 5 | Items, tax calculation, status |
| `clinical_history.rs` | 12 | Consultations, agenda, lab results |
| `settings.rs` | 6 | Get/save, null handling |
| `users.rs` | 15 | CRUD, password changes, validation |
| `dashboard.rs` | 11 | Stats, upcoming lists |
| `auth.rs` | 10 | Find user, audit log |
| `catalog.rs` | 10 | Species, breeds, analytes |
| **Total** | **89** | |

- **`test_helpers.rs`**: New module with temp database setup, migration runner, demo data cleanup, and fixture helpers (`insert_test_patient`, `insert_test_sample_type`, `insert_test_analyte`, `insert_test_reference_range`)

#### AI Improvements

- **AI interpretation cache** (`ai_cache.rs`):
  - In-memory cache with 24-hour TTL
  - Cache key: `sample_id` + hash of lab results
  - Auto-invalidation when results are updated
  - 8 unit tests for cache operations

- **Enhanced AI prompt** with clinical context:
  - Patient details (neutered, color, notes)
  - Recent consultation history (last 3)
  - Vaccination status (last 5)
  - Previous lab results for trend comparison (last 20)
  - Structured table format with emoji indicators
  - System message for expert veterinary context

- **TypeScript bindings**: Added `interpretLabResults` to `bindings.ts`

#### Unit Tests (133 new tests)

- **Repository mapping tests** (21): Patient, sample, lab result, surgery, vaccine, invoice mappers
- **AI command tests** (13): Prompt construction, response parsing
- **AI cache tests** (8): Set/get, TTL expiration, invalidation, hash functions
- **Auth tests** (2): Password hashing, role logic
- **PDF builder tests** (28): Layout, formatting, calculations
- **Error tests** (3): Display, serialization, conversions

#### Frontend Improvements

- **Zod validation schemas** for 3 forms:
  - `LoginPage.tsx`: Username/password validation
  - `NewInvoiceDialog.tsx`: Dynamic items with `useFieldArray`
  - `SampleDetailDialog.tsx`: Lab result validation

### Changed

- **CI pipeline now passes**: `cargo clippy -- -D warnings` succeeds
- **Test coverage**: Rust backend tests increased from 30 to 202 (+573%), TypeScript tests at 102
- **Version**: Updated to 0.3.0 in `package.json`, `Cargo.toml`, `tauri.conf.json`

### Test Infrastructure

- **Firebird Embedded**: Integration tests use temporary databases with unique names
- **Auto-cleanup**: Test databases are deleted after each test
- **Fixture helpers**: Reusable functions for test data setup
- **Generator management**: Properly reset Firebird generators between tests

## [0.1.0] - Previous releases

See [README.md](README.md) for information about prior milestones (Hito 1-4).
