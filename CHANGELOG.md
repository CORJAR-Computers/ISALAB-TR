# Changelog

All notable changes to ISALAB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
