# Changelog

All notable changes to ISALAB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.3.0] - 2026-08-04

### 🎯 Session Summary

Major quality improvement session focusing on testing, code quality, and AI integration:

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Rust tests | 30 | 163 | **+443%** |
| TypeScript tests | 102 | 102 | - |
| Clippy warnings | 28 | 0 | **-100%** |
| Repositories tested | 2 | 11 | **+450%** |
| AI features | Basic | Enhanced + Cached | ✅ |

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
- **Test coverage**: Rust backend tests increased from 30 to 163 (+443%), TypeScript tests at 102
- **Version**: Updated to 0.3.0 in `package.json`, `Cargo.toml`, `tauri.conf.json`

### Test Infrastructure

- **Firebird Embedded**: Integration tests use temporary databases with unique names
- **Auto-cleanup**: Test databases are deleted after each test
- **Fixture helpers**: Reusable functions for test data setup
- **Generator management**: Properly reset Firebird generators between tests

## [0.1.0] - Previous releases

See [README.md](README.md) for information about prior milestones (Hito 1-4).
