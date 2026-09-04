# ISALAB v0.6.0 — Lab module improvement analysis

**Repo:** https://github.com/CORJAR-Computers/ISALAB-TR
**Base:** v0.5.0 (released)
**Stack:** Tauri v2 (Rust) + React 19 + TypeScript + Vite + Tailwind v4 + shadcn/ui + TanStack Query + Firebird 5 Embedded (`rsfbclient`) + `printpdf`
**Status:** Proposal — approved by the team on <date>, pending implementation

---

## Current state (v0.5.0 shipped)

The lab module already has: critical values + delta checks (with manual WhatsApp alert), HIL quality + rejection workflow with per-sample event trail, CSV analyzer import + panels/batch entry, and an internal QC module (Westgard + Levey-Jennings). What it *lacks* relative to the candidates below:

| Capability | Status today |
|---|---|
| Analyzer connectivity | CSV import only (manual, file picker) |
| Lab orders | **None** — samples are created directly at reception, no requisition/ordering step |
| Email/SMTP | **None anywhere** in the codebase |
| Notification audit trail | Critical "Entendido" confirm is *not persisted*; WhatsApp is a manual `wa.me` deep link |
| Owner contact | `OWNERS.EMAIL` **already exists** — owner email is ready to use |
| Vet email | `USERS` has no email column (would need a migration) |
| Billing link | `INVOICE_ITEMS` exists but has **no sample/order FK** |
| Barcode scanning at reception | Labels generate Code128, but no scan-handler at reception |

---

## Evaluation of the three suggested candidates

### 1. HL7/ASTM analyzer integration — high complexity, *conditional* value

Point-of-care veterinary analyzers (Idexx Catalyst, Heska Element, Abaxis) do **not** expose open ASTM E1394 to end users — they integrate through vendor middleware (VetLab Station, VetConnect) or CSV/print exports. ASTM E1381/E1394 (CLSI LIS2-A2) is a real standard, and open-source LISes (OpenELIS ships generic ASTM + HL7 drivers) prove it's buildable — but in practice each analyzer's implementation is vendor-specific, and you cannot validate a serial driver without the actual instrument and its protocol documentation.

**The pragmatic path:** don't bet on one protocol. Build a small **`AnalyzerSource` abstraction** (trait: `poll() -> Vec<RawResult>`) with drivers, and ship the one that works today with the clinic's real export path:

- **Watched-folder auto-import** (S–M, ~3–5 days): reuse the existing CSV preview/mapping pipeline, add a folder watcher + an import queue UI with per-file status. CSV export *is* the de facto export path for these analyzers, so this delivers most of the value immediately with zero hardware risk.
- **ASTM E1381 serial receiver** (M–H, 1–2 weeks + hardware validation): generic frame parsing (ENQ/ACK, H/P/O/R/C/L records, checksum) is unit-testable in isolation, but bidirectional mode (host query + sample IDs) explodes in vendor-specific complexity. Ship only if the clinic confirms their analyzer supports it.
- **HL7 v2 ORU over MLLP** (M): framing is simple, parsing ORU^R01 is medium — but only useful if middleware/analyzer emits HL7.

**Verdict: include the abstraction + watched-folder in v0.6.0; keep ASTM/HL7 drivers behind it for a follow-up once hardware is confirmed.**

### 2. Lab orders — the biggest workflow gap, highest long-term value

Today there's no "what did the vet order" step: reception creates a sample directly. Adding orders transforms the workflow:

- New `LAB_ORDERS` table (patient, requesting vet, priority, ordered tests/panels, status: `SOLICITADA → RECIBIDA → EN_PROCESO → COMPLETADA / ANULADA`), created from a consultation or directly.
- **Accessioning** converts an order into one or more samples with the ordered panels **pre-filled** — this is exactly what panels (v0.5.0) were built to enable.
- Order-level TAT and test-utilization metrics (feeds the dashboard work already started).
- Optional billing link: `INVOICE_ITEMS.ORDER_ID` (nullable FK — migration, since existing invoice items/samples stay untouched).
- An "orden médica" requisition PDF reuses the existing PDF infra.

**Effort: M–H (~1–2 weeks). Value: high — it's the backbone every other feature (utilization, pre-analytical metrics, external lab) hangs off.**

### 3. Critical-value notifications by email — medium effort, high value, fully testable

No SMTP exists, but the schema is ready (`OWNERS.EMAIL`). This is the highest ROI/effort ratio of the three because it also closes a **regulatory-grade audit gap**: CLSI GP47 requires that critical-result notification be *logged with acknowledgment* — and today the "Entendido" button persists nothing.

Build a **notification module**:

- `NOTIFICATION_LOG` table (result_id, channel `WHATSAPP|EMAIL|SMS|PHONE`, recipient, sent_at, acked_at, acked_by) — *all* critical confirmations and sends get persisted, replacing the ephemeral dialog.
- Email channel via the `lettre` crate: SMTP settings in `CLINIC_SETTINGS` (host, port, TLS, user, app-password, from address) + an in-app **outbox queue with retry** (desktop app may be closed when a result lands).
- Recipients: owner (from `OWNERS.EMAIL`) and/or clinic/vet — add `EMAIL` to `USERS` via migration.
- Email complements rather than replaces WhatsApp/phone; the durable part is the audit trail + ack, which also enables a "critical values notification report".

**Effort: M (~1 week). Testable with any SMTP test account — no hardware dependency.**

---

## Other candidates worth bundling (small wins)

| Feature | Effort | Why |
|---|---|---|
| **Barcode scan at reception** | S | USB scanners act as keyboard; a scan-handler (code → open sample detail) reuses Code128 labels already generated. |
| **Persist critical confirmation** | S | Fold into the notification module (part of #3). |
| **External-lab referral workflow** | M | `ENVIADA_A_EXTERNO` state + external-lab profiles + results import back; very relevant for Colombian clinics that send to reference labs. |
| **Auto-verification rules** | M–H | Safety-critical rules engine; **defer to v0.7.0**. |
| **Reagent/inventory + QC lot tracking** | M | Natural extension of the QC module, but secondary. |

---

## Recommended v0.6.0 scope

Three features, executed in order, each as its own commit + migration + CI-green:

1. **Notification module (critical values)** — audit trail + ack + email channel via SMTP. *Foundation, ~1 week.*
2. **Lab orders** — order → accessioning → panels pre-filled → invoice link. *Workflow backbone, ~1–2 weeks.*
3. **Analyzer integration, pragmatic tier** — `AnalyzerSource` abstraction + watched-folder auto-import; ASTM/HL7 drivers as a documented follow-up pending hardware confirmation. *~3–5 days for the folder tier.*

**Total: ~3–4 weeks. Deferred: raw ASTM/HL7 drivers (needs hardware), autoverification, inventory (both natural v0.7.0 items).**

---

## Data-model notes for implementation

- `OWNERS.EMAIL` exists — no migration needed for owner email.
- `USERS` needs an `EMAIL` column for vet routing (migration).
- `INVOICE_ITEMS` needs a nullable `ORDER_ID` FK for billing (migration).
- Everything else is new tables (0017+) following the existing generator/trigger pattern (see `0004_user_audit_log.sql`, `0016_sample_events.sql`).
- Release mechanics: one feature per commit/tag; CI + release workflows already exist (tag `v0.6.0`, NSIS installer + auto-update).

---

## Sources

- CLSI GP47 — Management of Critical- and Significant-Risk Results (critical-value notification, logging, acknowledgment)
- CLSI LIS2-A2 (ASTM E1394) — point-to-point analyzer communication standard
- OpenELIS Global — ships generic ASTM + HL7 drivers (feasibility precedent)
- Idexx Catalyst / Heska Element / Abaxis — vendor middleware integration model for POC analyzers