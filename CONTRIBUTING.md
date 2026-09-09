# Contribuir a ISALAB

¡Gracias por tu interés en contribuir a ISALAB! Este documento describe cómo
preparar el entorno, qué esperar del flujo de desarrollo y cómo enviar tus
cambios para que se revisen rápido.

> ¿Encontraste un fallo de **seguridad**? Por favor **no** abras un issue
> público: sigue [SECURITY.md](./SECURITY.md).

## Código de conducta

Al participar aceptas cumplir el [Código de conducta](./CODE_OF_CONDUCT.md).
Sé respetuoso y constructivo.

## Requisitos del entorno

| Herramienta | Versión | Notas |
| --- | --- | --- |
| Node.js | **≥ 20** (recomendado 24) | El CI usa 24. Usa `nvm use` (ver `.nvmrc`). |
| npm | el incluido con Node | `package-lock.json` está trackeado. |
| Rust | **stable ≥ 1.77** | `rustup default stable`. |
| MSVC Build Tools | Windows | Target `x86_64-pc-windows-msvc` para compilar Tauri. |
| Firebird 5 Embedded | solo dev | Copia `fbclient.dll` en `src-tauri/binaries/firebird/` (ver README). |

> En Windows, ejecuta `npm run setup:target` para mover el `target/` de Cargo
> fuera del árbol del antivirus (detallado en el README).

## Primer arranque

```bash
npm install            # dependencias frontend
npm run icons          # genera src-tauri/icons (PNG + ICO)
npm run tauri:dev      # compila Rust + arranca la app con la UI
```

En el primer arranque la app crea `isalab.fdb`, aplica las migraciones
versionadas y siembra el catálogo y el usuario `admin` (cambia la contraseña
`admin123` en el primer login).

## Estructura del proyecto

```
src/                     # Frontend React 19 + TypeScript + Vite
├── bindings.ts          # ⚠ Generado por specta — NO editar a mano
├── lib/api.ts           # Wrappers tipados de los comandos Tauri
├── stores/               # Zustand (UI + sesión)
├── hooks/                # TanStack Query + eventos Firebird
├── components/ui/        # shadcn/ui (estilo new-york)
└── features/            # Módulos de dominio (patients, samples, lab-orders…)
src-tauri/               # Backend Rust (Tauri v2)
├── migrations/          # SQL versionado (0001…0021)
└── src/
    ├── commands/        # Comandos Tauri (capa fina, RBAC)
    ├── repositories/    # Acceso a datos (patrón Repository)
    ├── models/          # DTOs con derive specta::Type
    ├── pdf_templates/   # Generación de PDF (printpdf + firma PKCS#12)
    ├── db/              # Pool Firebird, migraciones, eventos POST_EVENT
    └── ...
```

## Flujo de trabajo recomendado

1. **Abre un issue** describiendo qué quieres cambiar (bug o mejora) para
   alinear antes de programar.
2. Crea una rama desde `master`:
   `git checkout -b feat/mi-mejora`.
3. Haz commits pequeños y atómicos siguiendo **Conventional Commits** (ver
   abajo).
4. Asegúrate de pasar todas las verificaciones locales (siguiente sección).
5. Abre un Pull Request contra `master` y completa la checklist del template.

## Verificaciones locales (antes de pushear)

El CI corre exactamente esto; pásalo en local para evitar ida y vuelta:

```bash
# Frontend
npm run lint                 # ESLint
npm run test                 # Vitest (120 tests)
npm run build                # tsc -b + vite build
npm run check:bindings       # ⚠ drift: bindings.ts vs comandos Rust

# Backend (Rust)
cd src-tauri
cargo fmt --all -- --check    # formato
cargo clippy -- -D warnings  # lints (warnings = error)
cargo test                   # 294 tests
cargo audit --ignore RUSTSEC-2026-0187 --ignore RUSTSEC-2023-0071
cd ..

# E2E (opcional, requiere navegador)
npx playwright install chromium
npm run test:e2e
```

### Bindings TypeScript (importante)

Los tipos del frontend (`src/bindings.ts`) se **generan automáticamente** con
`tauri-specta` al ejecutar `npm run tauri:dev` o `npm run tauri:build`. **Nunca**
los edites a mano: el CI verifica el drift (`npm run check:bindings`) y falla
si Rust y los bindings no coinciden.

- Si añades/cambias un comando o un modelo con `#[derive(specta::Type)]`,
  regénralos con `npm run tauri:dev` (o `tauri build`) y commitea el
  `bindings.ts` actualizado en el mismo PR.

### Si añades un comando Tauri

1. Define el comando en `src-tauri/src/commands/<modulo>.rs` con sus
   `#[tauri::command]` y permisos (`require_session` / `require_admin` /
   `require_vet_or_admin`).
2. Regístralo en `collect_commands!` dentro de `src-tauri/src/lib.rs` →
   `specta_builder()`.
3. Expón los tipos con `.typ::<…>()` si añades modelos nuevos.
4. Añade el wrapper tipado en `src/lib/api.ts`.
5. Crea el hook de TanStack Query en `src/hooks/queries/`.
6. Regenera `bindings.ts` (paso anterior).

### Si tocas el esquema de base de datos

- Añade una nueva migración `src-tauri/migrations/00NN_<nombre>.sql` (sigue el
  patrón `SET TERM` de las existentes).
- Las migraciones son **idempotentes y versionadas**; el arranque las aplica en
  orden y guarda el `schema_version`.
- Acompáñala de un test en `src-tauri/` cuando aplique (el repo tiene helpers en
  `src-tauri/src/test_helpers.rs`).

## Convenciones de commits (Conventional Commits)

Usamos [Conventional Commits](https://www.conventionalcommits.org/) para que el
CHANGELOG se genere de forma consistente:

```
<tipo>(<alcance opcional>): <descripción>

< cuerpo opcional, explica el porqué >
```

Tipos habituales:

| Tipo | Uso |
| --- | --- |
| `feat` | Nueva funcionalidad |
| `fix` | Corrección de bug |
| `refactor` | Reorganización sin cambio de comportamiento |
| `perf` | Mejora de rendimiento |
| `docs` | Solo documentación (README, CHANGELOG, SECURITY… ) |
| `test` | Añadir/corregir tests |
| `ci` | Cambios de pipeline/CI/release |
| `chore` | Mantenimiento (dependencias, scripts) |
| `style` | Formato (sin cambio lógico) |

Ejemplo:

```
 feat(lab-orders): permite anular una orden desde la mesa de trabajo
```

> Los **releases** se marcan con un tag `vX.Y.Z` (ej. `v0.6.1`); eso dispara el
> workflow de release automáticamente. **No** publiques tags si no estás seguro.

## Estilo de código

- **TypeScript**: ESLint con `eslint-plugin-react-hooks` y
  `typescript-eslint`. Hooks: `rules-of-hooks` en error,
  `exhaustive-deps` en warn. Preferimos `const` y funciones flecha.
- **Rust**: `cargo fmt` + `clippy -D warnings`. Evita `unwrap()`/`expect()` en
  rutas de producción (reserva el `panic!` para invariantes imposibles en
  tests, como en `pdf_templates/builder.rs`).
- **CSS**: Tailwind v4 con tokens OKLCH en `src/index.css`. No uses colores
  indigo/azul sueltos: usa los tokens (`bg-primary`, `text-foreground`…).

## Reportar bugs

Abre un issue con la plantilla **Reporte de bug** (`.github/ISSUE_TEMPLATE/`).
Incluye versión de la app, SO, pasos para reproducir y resultado esperado vs.
real. Cuanto más reproducible, más rápido lo abordamos.

## Agradecimientos

Toda contribución cuenta. Reconocemos a quienes contribuyen en las release
notes del release correspondiente. ¡Gracias por hacer ISALAB mejor!
