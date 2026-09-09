# Política de Seguridad de ISALAB

> **Resumen (EN):** ISALAB is a desktop application (Tauri v2 + Rust) for
> veterinary laboratories. It stores sensitive clinical data and handles local
> authentication, secret encryption (Windows DPAPI) and an auto-updater signed
> with minisign. Please report security issues **privately** (see below). Do
> not open public issues for vulnerabilities.

---

## Reportar una vulnerabilidad

Agradecemos los reportes responsables de seguridad. **No abras un issue
público** para una vulnerabilidad: usa el canal privado para que podamos
evaluar y remediar antes de cualquier divulgación.

### Canal preferido: GitHub Private Vulnerability Reporting

1. Ve a la pestaña **Security** del repositorio:
   <https://github.com/CORJAR-Computers/ISALAB-TR/security>
2. Pulsa **Report a vulnerability** y completa el formulario.

Este canal genera un reporte privado visible solo para los mantenedores y te
permite seguir la discusión y colaborar en la corrección.

### Canal alternativo

Si no puedes usar GitHub, escribe a los mantenedores a través de la
información de contacto del perfil de la organización `CORJAR-Computers`.

## Alcance

### Dentro del alcance

- El código de la aplicación ISALAB (`src/` frontend y `src-tauri/src`
  backend en Rust).
- El manejo de **secreto cifrado con DPAPI** (claves de IA/SMTP, certificado
  PKCS#12) en `src-tauri/src/crypto.rs`.
- El **sistema de autenticación local** (Argon2id, RBAC, rate limiting de
  login) en `src-tauri/src/auth.rs` y `src-tauri/src/commands/auth.rs`.
- El **auto-updater**: verificación de firma minisign, manifiesto `latest.json`
  y el flujo de re-firmado tras SignPath en `.github/workflows/release.yml`.
- La generación y **firma digital PKCS#12** de reportes PDF
  (`src-tauri/src/pdf_templates/signing.rs`).
- Las migraciones SQL y el seed de la base de datos Firebird
  (`src-tauri/migrations/`, `src-tauri/src/db/seed.rs`).
- El pipeline de CI/release y la gestión de secretos de firma.

### Fuera del alcance (reportar al upstream)

- Vulnerabilidades en **Tauri** mismo:
  <https://github.com/tauri-apps/tauri/security>
- Vulnerabilidades en **Firebird SQL**:
  <https://www.firebirdsql.org/en/security/>
- Vulnerabilidades en dependencias Rust (revisa `cargo audit`): reportar a
  través de <https://rustsec.org/advisories/> y, si aplica, abre un issue aquí
  para acotar/actualizar la dependencia afectada.

> **Nota sobre dependencias con advisories conocidos:** el CI ejecuta
> `cargo audit` ignorando dos advisories transitorios sin fix disponible
> (`RUSTSEC-2026-0187` lopdf vía `pdf_signer`, y `RUSTSEC-2023-0071` rsa,
> Marvin Attack) documentados en `CHANGELOG.md`. Un reporte sobre esos dos no
> aporta información nueva; cualquier **otro** advisory sí debe reportarse.

## Qué incluir en el reporte

Para que podamos reproducir y priorizar rápido, incluye:

- **Versión de ISALAB** (visible en *Acerca de* dentro de la app o en
  `tauri.conf.json`/`package.json`).
- **Sistema operativo** y versión (la app es Windows x64 en producción).
- **Pasos para reproducir** el problema (o un PoC mínimo).
- **Impacto estimado**: qué podría hacer un atacante (escalar privilegios,
  leer datos clínicos, eludir RBAC, ejecutar código, falsificar una
  actualización, etc.).
- Si lo tienes, una **trace** o el mensaje exacto del error.

## Tiempos de respuesta objetivo

- **Acuse de recibo**: en hasta **72 horas**.
- **Evaluación inicial y severidad (CVSS orientativo)**: en hasta **7 días**.
- **Corrección o mitigación**: según severidad; publicaremos un **aviso
  advisory** en la pestaña Security con crédito al reportero (si lo desea) una
  vez disponible la versión parcheada.

No podemos garantizar tiempos estrictos (proyecto mantenido por un equipo
pequeño), pero nos comprometemos a mantener comunicación abierta durante todo
el proceso.

## Divulgación coordinada

- Te pedimos **no divulgar** públicamente el detalle de la vulnerabilidad hasta
  que exista una versión corregida y se haya coordinado un anuncio.
- Publicaremos los advisories resueltos en la pestaña **Security** del repo y,
  para problemas de actualización, en las **release notes** del primer release
  que incluya la corrección.

## Safe Harbor

Consideramos la investigación de seguridad de buena fe realizada con respeto a
esta política y a la privacidad de los usuarios como un aporte valioso. No
tomaremos acciones legales contra quienes reporten de forma responsable y
confidencial. Esperamos reciproquen no dañando datos, interrumpiendo servicio
ni accediendo a información de terceros más allá de lo necesario para
demostrar el problema.

## Prácticas de seguridad implementadas

Esta sección ayuda a contextualizar los reportes y a no duplicar lo ya
mitigado:

- **Autenticación**: hashes Argon2id (con salt aleatorio) en
  `src-tauri/src/auth.rs`; la verificación es local (el hash nunca sale del
  dispositivo).
- **RBAC**: los **117 comandos** Tauri exigen sesión activa
  (`require_session`); las mutaciones críticas exigen rol `ADMIN`
  (`require_admin`); las consultas/cirugías exigen `VETERINARIO` o `ADMIN`.
- **Rate limiting de login**: 5 intentos fallidos bloquean al usuario 5 min.
- **Cifrado de secretos**: la clave de IA (Groq) y otros secretos se cifran
  con **DPAPI de Windows** (formato `enc:v1:<base64>`), ligados al usuario de
  Windows; los valores legacy en texto plano se re-cifran al primer acceso.
- **Auditoría**: `USER_AUDIT_LOG` registra inicios/cierres de sesión, intentos
  fallidos, cambios de contraseña/configuración y transiciones de estado.
- **CSP**: `script-src 'self'` (sin `eval`), configurado en `tauri.conf.json`.
- **Auto-updater firmado**: cada instalador se firma con **minisign**; la
  clave pública embebida verifica la firma antes de instalar. Actualizaciones
  no firmadas **no se instalan**.
- **Firma de PDF**: los reportes pueden firmarse digitalmente con certificado
  PKCS#12 (.p12/.pfx).
- **Base de datos local**: `isalab.fdb` vive en `app_data` y nunca se commitea;
  el motor Firebird 5 Embedded se bundlea en el instalador.
- **CI**: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo audit`
  (RustSec), ESLint, Vitest (120 tests), Playwright E2E y verificación de
  drift de bindings TypeScript.

> Si tu reporte afecta a cualquiera de estas mitigaciones, indícalo
> explícitamente para acelerar la evaluación.
