## Resumen

<!-- Qué hace este PR y por qué. 1-3 frases. Si cierra un issue, enlázalo: -->
<!-- Closes #123 -->

## Tipo de cambio

- [ ] `feat` — nueva funcionalidad
- [ ] `fix` — corrección de bug
- [ ] `refactor` — reorganización sin cambio de comportamiento
- [ ] `perf` — mejora de rendimiento
- [ ] `docs` — solo documentación
- [ ] `test` — añade/corrige tests
- [ ] `ci` — pipeline/CI/release
- [ ] `chore` — mantenimiento (dependencias, scripts)
- [ ] `style` — formato (sin cambio lógico)

## Checklist — Frontend (`src/`)

- [ ] `npm run lint` pasa sin errores.
- [ ] `npm run test` (Vitest) pasa.
- [ ] `npm run build` (`tsc -b && vite build`) pasa.
- [ ] **No** edité `src/bindings.ts` a mano (lo regeneré con `tauri dev`/`build`).
- [ ] Si añadí/cambié un comando o modelo Rust → corrí `npm run check:bindings`
      y el resultado está commiteado.

## Checklist — Backend (`src-tauri/`)

- [ ] `cargo fmt --all -- --check` pasa.
- [ ] `cargo clippy -- -D warnings` pasa.
- [ ] `cargo test` pasa.
- [ ] Si añadí un comando Tauri → lo registré en `collect_commands!` (`lib.rs`)
      y expuse los tipos con `.typ::<…>()` si corresponde.
- [ ] Si añadí un comando → añadí el wrapper en `src/lib/api.ts` y el hook de
      TanStack Query en `src/hooks/queries/`.
- [ ] Apliqué el permiso adecuado (`require_session` / `require_admin` /
      `require_vet_or_admin`) según la criticidad.
- [ ] Si toqué el esquema → añadí una migración `00NN_<nombre>.sql` siguiendo
      el patrón `SET TERM` existente.

## Checklist — E2E y seguridad

- [ ] Si el cambio toca un flujo crítico (login, muestras, resultados críticos,
      facturación, reportes, auto-update) → verifiqué el smoke E2E
      (`npm run test:e2e`).
- [ ] No introduje secretos ni claves privadas en el diff.
- [ ] Si el cambio afecta a seguridad (auth, crypto, RBAC, updater, firma PDF)
      → revisé [SECURITY.md](../SECURITY.md) y lo reflejé si procede.

## Notas para el revisor

<!-- Cualquier detalle: decisiones de diseño, tradeoffs, qué probar manualmente
en el build de Windows, dependencias transitivas afectadas, etc. -->

## Capturas / evidencia (si aplica)

<!-- Pega capturas o gifs si el cambio es visual (UI, PDFs, banner de
actualización…). -->
