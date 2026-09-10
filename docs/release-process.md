# Proceso de release de ISALAB

> Cómo preparar, verificar y publicar una nueva versión. Referencia rápida
> de comandos al final (§9). El texto del popup que ven los usuarios al
> actualizarse vive en `.github/updater-notes.md`; las notas completas de
> cada versión, en `CHANGELOG.md`.

## 0. Requisitos previos

- `master` verde en CI (fmt, clippy, tests Rust sobre Firebird real,
  frontend, E2E) y sincronizado con `origin/master`.
- El job Backend de CI tarda ~1h50m: no taggear sobre un push cuyo CI
  todavía corre.
- Acceso que permita empujar el tag y editar el release (branch protection
  exige los 3 checks; con bypass de admin el push pasa y los checks corren
  después).

## 1. Elegir el número de versión

SemVer sobre lo acumulado en `[Unreleased]` del CHANGELOG:

- solo correcciones → patch (`1.1.1`)
- funcionalidad nueva → minor (`1.2.0`) — v1.1.0 fue minor por las
  referencias de referencia manuales
- cambio incompatible → major

## 2. Subir la versión (6 sitios)

`package.json`, `package-lock.json` (raíz y paquete `""`),
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` y `src-tauri/Cargo.lock`
(este último se reescribe solo con `cargo check`):

```bash
sed -i 's/"version": "1.1.0"/"version": "1.2.0"/' package.json package-lock.json src-tauri/tauri.conf.json
sed -i 's/^version = "1.1.0"/version = "1.2.0"/' src-tauri/Cargo.toml
(cd src-tauri && cargo check -q)
```

Verificación: no debe quedar rastro de la versión vieja fuera de historia:

```bash
grep -rn '"version": "1.1.0"\|^version = "1.1.0"' package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
```

## 3. CHANGELOG + texto del popup

- **`CHANGELOG.md`**: convertir `## [Unreleased]` en
  `## [1.2.0] - YYYY-MM-DD`, con un párrafo-resumen del release arriba
  (formato de v1.0.0/v1.1.0: negrita inicial, tabla de métricas si aplica).
- **`.github/updater-notes.md`**: español, **primera línea exactamente
  `ISALAB 1.2.0`**, una viñeta por novedad (4–6 líneas en total). El
  workflow valida esa primera línea: si quedó vieja, la build usa el texto
  genérico «ISALAB v1.2.0» y emite un `::warning::` en el resumen del run —
  la release no se bloquea, pero el popup sale sin detalle.

## 4. Commit de release, tag y push

```bash
git add -A && git commit -m "chore(release): prepare v1.2.0, ..."
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin master v1.2.0
```

Empujar el **tag** dispara `.github/workflows/release.yml`: build NSIS
nativo en el runner self-hosted → firma minisign del instalador
(`tauri signer sign`) → generación de `latest.json` (notas desde
`.github/updater-notes.md`, con validación de versión) → **GitHub Release
en borrador** con el instalador, el `.sig` y el manifiesto. Si la máquina
del runner está apagada: fallback cross-compile con `workflow_dispatch`
(input `tag`).

## 5. Draft: revisión pre-publicación

Los usuarios no ven nada todavía; el endpoint del updater sigue sirviendo
la versión anterior. El HEAD anónimo de los assets da 404 — es lo esperado
(en un borrador son privados); para revisarlos usar `gh` (autenticado):

1. Esperar el run de Release (~50 min) y confirmar los tres assets en
   estado `uploaded`.
2. **Notas de la página**: reemplazar el cuerpo autogenerado
   (`generate_release_notes` deja «What's Changed» + enlace de
   comparación) por las notas en español de la versión — estilo house:
   párrafo de apertura en negrita, secciones planas sin emojis, y el enlace
   de comparación al final. Fuente: la sección recién escrita del
   CHANGELOG, traducida a lenguaje de usuario.
3. **Auditar el feed del draft**: descargar `latest.json` con `gh` y
   verificar `version`, `notes`, `url` y que la `signature` siga siendo la
   del instalador (el `.sig` es la forma base64 de la firma embebida).

## 6. Publicar y verificar en caliente

Publicar **es** activar el feed: en ese instante el release se vuelve
visible, recibe la insignia Latest, y
`.../releases/latest/download/latest.json` pasa a servir la nueva versión
a toda la base instalada.

```bash
gh release edit v1.2.0 --draft=false
npm run verify-updater
```

`npm run verify-updater` es el dry-run de extremo a extremo
(`scripts/verify-updater.mjs`): fetch del endpoint, comparación semver,
texto del popup que vería el usuario, descarga del instalador y
verificación minisign con la pubkey embebida en `tauri.conf.json`
(algoritmo exacto de `minisign-verify`: Ed25519 sobre blake2b-512 del
instalador + firma global sobre la firma del archivo y el trusted
comment). Debe terminar con todos los checks en ✅ y exit 0, en ~2 minutos.

Notas sobre su salida: contra el feed recién publicado y el repo ya con el
bump, el paso de versión concluye «misma versión» (y valida el resto
igual); contra un feed viejo concluye «hay actualización». Si el feed
estuviera roto (404, firma inválida), falla con exit 1 — en ese caso,
reconvertir el release a borrador (`gh release edit v1.2.0 --draft=true`)
deja de exponerlo de inmediato.

## 7. Post-release

- Vigilar adopción:
  `gh api repos/CORJAR-Computers/ISALAB-TR/releases/latest --jq '.assets[] | {name, downloadCount}'`.
- Las migraciones nuevas (p. ej. 0022/0023/0024) corren solas en el primer
  arranque de cada cliente; la 0023 poda `EVENT_LOG` >30 días, así que los
  backups locales encogen tras la primera ejecución de la nueva versión.
- Si se publica una corrección del proceso o del CHANGELOG, es un commit
  normal en `master`; no toca el tag ya construido.

## 8. Qué control existe y por qué

| Control | Qué garantiza |
|---|---|
| Firma minisign (`.sig` + `signature` del feed) | El binario que instala el cliente salió del pipeline; la clave privada solo vive en los secretos del repo/runner. Verificada contra la pubkey embebida en `tauri.conf.json`. |
| Primera línea de `updater-notes.md` | El popup muestra `notes` textual; el workflow valida la versión y cae al genérico con warning si está vieja. |
| `npm run verify-updater` | La cadena completa (feed coherente + instalador firmado) funciona tal como la vivirá un cliente. |
| Release en borrador | El build puede revisarse sin exponer nada; publicar es un gesto explícito. |

## 9. Referencia rápida

```bash
# 0. master verde y sincronizado
git checkout master && git pull
gh run list --branch master --limit 1

# 1–3. bump, CHANGELOG y popup (ejemplo 1.1.0 → 1.2.0)
sed -i 's/"version": "1.1.0"/"version": "1.2.0"/' package.json package-lock.json src-tauri/tauri.conf.json
sed -i 's/^version = "1.1.0"/version = "1.2.0"/' src-tauri/Cargo.toml
(cd src-tauri && cargo check -q)
$EDITOR CHANGELOG.md .github/updater-notes.md

# 4. commit + tag + push (el tag dispara el build)
git add -A && git commit -m "chore(release): prepare v1.2.0"
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin master v1.2.0

# 5. draft: esperar build, notas de página, auditoría del feed
gh run watch   # workflow Release
gh release edit v1.2.0 --title "v1.2.0" --notes-file notas-v1.2.0.md
gh release download v1.2.0 --pattern "latest.json" && cat latest.json

# 6. publicar y verificar en caliente
gh release edit v1.2.0 --draft=false
npm run verify-updater
```
