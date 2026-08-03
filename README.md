# ISALAB · Laboratorio Veterinario

Sistema de escritorio moderno para **laboratorios veterinarios** (inspirado en
[ISALAB](https://github.com/CORJAR-Computers/ISALAB.git)): historiales clínicos
multiespecie, muestras analíticas con trazabilidad, vacunación, reportes PDF
firmados y facturación.

## Stack

| Capa      | Tecnología |
| --------- | ---------- |
| Shell     | Tauri v2 (Rust) |
| Base de datos | Firebird 5.0 **Embedded** (`rsfbclient` 0.27, `dynamic_loading`) |
| Frontend  | React 19 + TypeScript + Vite |
| Estilos   | Tailwind CSS v4 + shadcn/ui |
| Estado    | Zustand + TanStack Query |
| IPC type-safe | specta / tauri-specta (`src/bindings.ts` auto-generado) |
| PDF       | `printpdf` (server-side, hito 2) |

## Requisitos

- Rust **stable** (≥ 1.77) con target `x86_64-pc-windows-msvc` + MSVC Build Tools
- Node.js ≥ 20 y npm
- **Firebird 5 Embedded**: copia `fbclient.dll` a
  `src-tauri/binaries/firebird/` (ver [README de binarios](src-tauri/binaries/firebird/README.md))

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
5. Siembra la contraseña del usuario `admin` (ver [Autenticación](#autenticación)).

> La base de datos nunca se commitea (`.gitignore`). Cada instalación la crea
> localmente.

## Autenticación y Seguridad

> [!CAUTION]
> **IMPORTANTE**: Al primer arranque el usuario **`admin`** queda habilitado con la contraseña por defecto **`admin123`** (Argon2id). Por razones de seguridad en entorno clínico, **debes cambiar esta contraseña inmediatamente** en tu primer inicio de sesión desde el módulo de Usuarios o Configuración.

- La sesión es local y única (una ventana, un operador a la vez).
- Control de acceso basado en roles (RBAC: `ADMIN`, `VETERINARIO`, `AUXILIAR`) enforced en los comandos nativos de Rust.
- Todos los comandos requieren sesión activa; las mutaciones críticas requieren rol `ADMIN`.
- **Auditoría**: tabla `USER_AUDIT_LOG` registra inicios/cierres de sesión, intentos fallidos de login, cambios de contraseña, creación de usuarios, cambios de configuración y transiciones de estado en muestras, facturas, consultas y cirugías.
- **CSP**: Content Security Policy configurado (`script-src 'self'`, sin eval).
- **Tests**: unit tests en Rust (`cargo test`) + Vitest en frontend (`npm test`).

## Estructura

```
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
- [x] **Hito 2**: reportes PDF con `printpdf` 0.12 (firma gráfica; PKCS#12
  pendiente), laboratorio (mesa de trabajo, resultados por analito con
  validación de rangos), configuración de la clínica y autenticación local
  (Argon2id, roles ADMIN/VETERINARIO/AUXILIAR).
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
