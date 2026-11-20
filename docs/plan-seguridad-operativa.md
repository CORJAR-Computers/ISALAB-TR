# Plan de implementación — Seguridad operativa: bloqueo de login, autobloqueo y re-autenticación

**Estado:** propuesta (no implementada)
**Base:** schema `0036` · login con Argon2id contra `USERS`; bloqueo por intentos **en memoria del proceso** (`state.login_attempts`, 5 intentos / 5 min, se pierde al reiniciar y no se comparte entre las 3 PCs); `useSessionTimeout` cierra la sesión a los **15 minutos fijos** sin pantalla de desbloqueo; `USER_AUDIT_LOG` registra `LOGIN`/`LOGIN_FAILED`/`LOGOUT`; ninguna acción sensible pide credenciales.
**Objetivo:** que la identidad detrás de cada operación con dinero sea verificable: un bloqueo por intentos que sobreviva al reinicio y valga en las tres PCs, un autobloqueo con desbloqueo (sin perder el contexto) y una **re-autenticación** obligatoria antes de anular un pago, anular una factura o mover caja a mano.

> **Numeración reservada:** `0043_security.sql`. Orden acordado: abonos/caja (`0036`), receta (`0037`–`0038`), recordatorios (`0039`), lotes (`0040`), cartera (`0041`), signos vitales y problemas (`0042`) y seguridad operativa (`0043`). **No depende de `0037`–`0042`**: solo toca `USERS` y `CLINIC_SETTINGS`, ambas creadas en `0001_initial_schema.sql`. Si se implementa antes, se renumera antes de crear la migración (no se deja un hueco).

---

## 1. Alcance

### 1.1 Entra

| Pieza | Contenido |
| --- | --- |
| Bloqueo de login | Contador persistido por usuario, umbral y duración configurables, mensaje con tiempo restante |
| Desbloqueo | Automático al expirar; manual por ADMIN desde Usuarios, con auditoría |
| Sesión bloqueable | `lock_session`/`unlock_session`; la sesión se conserva, la vista se retoma donde estaba |
| Autobloqueo | Por inactividad configurable, con aviso previo y cuenta atrás |
| Defensa en backend | Un comando de una sesión bloqueada o vencida se rechaza aunque la UI no lo bloquee |
| Re-autenticación | Contraseña propia + token de un solo uso y vida corta para acciones sensibles |
| Acciones sensibles | Anular pago, anular factura, gasto/retiro/ajuste de caja |
| Configuración | Claves `security.*` en `CLINIC_SETTINGS` y sección **Seguridad** (ADMIN) |
| Auditoría | `LOGIN_LOCKED`, `USER_UNLOCKED`, `SESSION_LOCKED`, `SESSION_UNLOCKED`, `SESSION_UNLOCK_FAILED`, `REAUTH_GRANTED`, `REAUTH_DENIED` |

### 1.2 No entra (MVP)

- Segundo factor (TOTP, SMS, llave física) y biometría.
- Conectividad con directorio (LDAP/AD) o inicio de sesión por dominio.
- Caducidad obligatoria de contraseña, historial de hashes o política de complejidad (hoy solo `MUST_CHANGE_PASSWORD`).
- Bloqueo por IP o por equipo: la app es de escritorio y la sesión es local; el contador vive por usuario.
- Cifrado de la base o del archivo local: es otra iniciativa (hoy el canal remoto usa SRP).
- PIN corto de 4 dígitos para desbloquear: se usa la contraseña real, sin un segundo secreto más débil.
- Re-autenticación en acciones clínicas (firmar resultado, prescribir): posible fase futura, mismo mecanismo.

### 1.3 Principios

1. **El backend es la autoridad.** Ocultar la UI no es seguridad: una sesión bloqueada debe fallar en `require_session`, no solo en React.
2. **El bloqueo se comparte.** Con tres PCs sobre la misma base, un contador en memoria deja 15 intentos por minuto (5 por PC) y se reinicia al reabrir la app. El contador vive en `USERS`.
3. **Bloquear no es cerrar.** Al bloquear se conserva usuario y contexto; se cierra sesión solo al elegir "Cambiar de usuario" o cuando la cuenta se bloquea por intentos.
4. **Un token, una acción, un usuario.** La autorización de una acción sensible no sirve para otra, no se reutiliza y caduca.
5. **Todo intento deja rastro**, exitoso o no, sin guardar contraseñas ni tokens en la auditoría.
6. **Sin fricción innecesaria:** el token dura lo suficiente para no pedir la contraseña dos veces por la misma operación, y el aviso de inactividad permite seguir trabajando con un clic.

---

## 2. Modelo de datos — migración `0043_security.sql`

### 2.1 Contador y bloqueo en `USERS`

```sql
ALTER TABLE USERS ADD FAILED_ATTEMPTS INTEGER DEFAULT 0 NOT NULL;
-- NULL = sin bloqueo. Con valor, el login se rechaza hasta esa hora.
ALTER TABLE USERS ADD LOCKED_UNTIL TIMESTAMP;
ALTER TABLE USERS ADD LAST_FAILED_AT TIMESTAMP;
ALTER TABLE USERS ADD LAST_LOGIN_AT TIMESTAMP;

CREATE INDEX IX_USERS_LOCKED ON USERS (LOCKED_UNTIL);
```

- `FAILED_ATTEMPTS` se incrementa de forma **atómica** en la propia sentencia (`SET FAILED_ATTEMPTS = FAILED_ATTEMPTS + 1`), para que dos PCs fallando a la vez no se pisen el contador.
- Al alcanzar el umbral, la misma transacción fija `LOCKED_UNTIL = DATEADD(? MINUTE TO CURRENT_TIMESTAMP)` y deja `FAILED_ATTEMPTS = 0` (el bloqueo ya no necesita el contador; al expirar se empieza de cero).
- Un login correcto limpia `FAILED_ATTEMPTS`, `LOCKED_UNTIL` y `LAST_FAILED_AT`.
- `LAST_LOGIN_AT` es informativo para el ADMIN (último acceso), no controla nada.

### 2.2 Política de seguridad en `CLINIC_SETTINGS`

Cinco claves nuevas, siguiendo el patrón plano de `SETTING_KEYS` (el arreglo pasa de 20 a 25 entradas):

| Clave | Default | Uso |
| --- | --- | --- |
| `security.lockout_max_attempts` | `5` | Intentos fallidos antes de bloquear |
| `security.lockout_minutes` | `15` | Duración del bloqueo de login |
| `security.idle_lock_minutes` | `15` | Inactividad antes de bloquear la sesión (mínimo 1, sin desactivar) |
| `security.idle_warning_seconds` | `60` | Antelación del aviso antes de bloquear (0 = sin aviso) |
| `security.reauth_minutes` | `2` | Vigencia del token de una acción sensible |

`ClinicSettings` gana los cinco campos como `Option` (sin valor = default) y un struct resuelto:

```rust
pub struct SecurityPolicy {
    pub lockout_max_attempts: u32,   // 5
    pub lockout_minutes: u32,        // 15
    pub idle_lock_minutes: u32,      // 15
    pub idle_warning_seconds: u32,   // 60
    pub reauth_minutes: u32,         // 2
}
```

`security_policy(conn)` resuelve defaults y acota rangos (1–20 intentos, 1–120 minutos de bloqueo, 1–240 de inactividad, 0–300 s de aviso, 1–15 min de token). La política se lee **al iniciar sesión** y se guarda en la sesión activa: cambiar la inactividad aplica desde el siguiente login (documentado en la UI), lo que evita una consulta por comando.

### 2.3 Lo que **no** se guarda

- Tokens de re-autenticación: viven solo en memoria (`AppState`), no en BD ni en logs.
- Contraseñas: nunca, ni en la auditoría ni en el detalle de un fallo (`LOGIN_FAILED` sigue guardando solo el motivo).
- Intentos de usuarios inexistentes: se mantienen en memoria (`login_attempts` actual), porque la tabla no tiene a quién apuntar. El mensaje al usuario es siempre el genérico.

---

## 3. Backend (Rust)

### 3.1 Estado de sesión

`AppState.session` deja de ser `Mutex<Option<SessionUser>>` y pasa a:

```rust
pub struct ActiveSession {
    pub user: SessionUser,
    pub locked: bool,
    pub last_activity: Instant,
    pub idle_timeout_secs: u64,    // política congelada al iniciar
    pub reauth: Option<ReauthGrant>,
}

pub struct ReauthGrant {
    pub action: SensitiveAction,
    pub token: String,             // 32 bytes aleatorios en hex
    pub expires_at: Instant,
    pub user_id: i32,
}
```

`require_session` (y por tanto todos los `require_*`) gana tres comprobaciones antes de devolver el usuario:

1. sin sesión → `Forbidden("Inicia sesión para continuar")` (igual que hoy);
2. sesión vencida por inactividad → se marca bloqueada y devuelve `Forbidden("La sesión se bloqueó por inactividad")`;
3. sesión bloqueada → `Forbidden("Sesión bloqueada: desbloquea con tu contraseña")`.

Si pasa, actualiza `last_activity`. Esto convierte el temporizador de React en una comodidad, no en la única barrera.

### 3.2 Comandos

| Comando | Capacidad | Notas |
| --- | --- | --- |
| `login(input)` | pública | Misma firma; ahora usa el contador en BD y audita `LOGIN_LOCKED` |
| `logout()` | sesión | Sin cambios; limpia la sesión activa y su grant |
| `get_session()` | pública | **Cambia el retorno** a `SessionState` (ver §4.1) |
| `lock_session(reason?)` | sesión | Marca bloqueada; audita `SESSION_LOCKED` |
| `unlock_session(password)` | sesión bloqueada | Verifica contraseña, cuenta fallos, audita `SESSION_UNLOCKED`/`SESSION_UNLOCK_FAILED` |
| `authorize_sensitive_action(action, password)` | sesión | Devuelve `ReauthTokenInfo { token, action, expiresAt }`; audita `REAUTH_GRANTED`/`REAUTH_DENIED` |
| `unlock_user(user_id)` | ADMIN | Limpia bloqueo y contador; audita `USER_UNLOCKED` |
| `list_users()` | ADMIN | Añade `failedAttempts`, `lockedUntil`, `lastLoginAt` a `UserListItem` |
| `void_payment(paymentId, reason, reauthToken)` | `can_bill` + token | Rechaza sin token válido de `VOID_PAYMENT` |
| `set_invoice_status(id, status, reauthToken)` | vet/admin + token | Token de `CANCEL_INVOICE` |
| `add_cash_movement(input, reauthToken)` | `can_bill` + token | Token de `CASH_ADJUSTMENT` |

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensitiveAction { VoidPayment, CancelInvoice, CashAdjustment }
```

`ReauthTokenInfo` viaja al frontend solo en memoria; el comando sensible lo recibe como string y compara contra el grant **del usuario activo, para esa acción y sin expirar**. Se consume al usarlo (un solo uso). Comparación en tiempo constante no es crítica aquí (el token es aleatorio de 256 bits y el atacante ya está autenticado), pero se documenta.

### 3.3 Errores distinguibles

`AppError` gana:

```rust
ReauthRequired(String),   // serializado con type = "REAUTH_REQUIRED" y la acción en data
```

El frontend lo usa para abrir el diálogo sin adivinar por el texto. Un token inválido o caducado responde `ReauthRequired` de nuevo (nuevo intento), no `Forbidden`: el usuario puede resolverlo con su contraseña.

### 3.4 Reglas de negocio

- **Login:** si `LOCKED_UNTIL > CURRENT_TIMESTAMP`, rechazar con el tiempo restante redondeado a segundos, sin verificar la contraseña (evita gastar Argon2 y filtra menos). Al expirar, el bloqueo se limpia en el siguiente intento.
- **Umbral:** `FAILED_ATTEMPTS + 1 >= max_attempts` fija el bloqueo. El contador se reinicia al fijarlo.
- **Usuario inactivo:** no incrementa contador en BD (no hay login posible), pero sí se audita; se mantiene el mensaje actual.
- **Desbloqueo administrativo:** solo ADMIN; no cambia la contraseña ni obliga a cambiarla.
- **Autobloqueo:** el vencimiento se evalúa en cada comando (`now - last_activity > idle_timeout`). La UI avisa antes; si el proceso estuvo cerrado, al reabrir no hay sesión (el estado vive en memoria) y se pide login.
- **Re-auth:** la contraseña se verifica contra el usuario activo con Argon2; los fallos cuentan para el **mismo** contador de bloqueo de `USERS`. Tras `max_attempts` fallos desde la pantalla de desbloqueo, la sesión se cierra y la cuenta se bloquea.
- **Token:** caduca a los `reauth_minutes`; pedir otro invalida el anterior; `logout`/`lock_session` lo borran.
- **Auditoría:** una sola fila por evento; el detalle incluye la acción y el identificador (pago/factura/sesión), nunca la contraseña ni el token.
- **Caja:** el ajuste manual (`GASTO`/`RETIRO`/`AJUSTE`) exige token igual que la anulación; un ajuste post-cierre futuro (si se implementa) reutiliza `CASH_ADJUSTMENT`.

### 3.5 Archivos

```text
src-tauri/src/security.rs                    SensitiveAction, token, política resuelta
src-tauri/src/models/security.rs             SessionState, ReauthTokenInfo, SecurityPolicy
src-tauri/src/models/auth.rs                 UserListItem + campos de bloqueo
src-tauri/src/repositories/auth.rs           incremento atómico, bloqueo, desbloqueo, listado
src-tauri/src/commands/auth.rs               login, lock, unlock, authorize, unlock_user
src-tauri/src/state.rs                       ActiveSession, política congelada, grant
src-tauri/src/auth.rs                        require_session con bloqueo/vencimiento
src-tauri/src/error.rs                       variante ReauthRequired
src-tauri/src/commands/payments.rs           void_payment con token
src-tauri/src/commands/invoices.rs           set_invoice_status con token
src-tauri/src/commands/cash.rs               add_cash_movement con token
src-tauri/src/repositories/settings.rs       claves security.* y security_policy()
```

---

## 4. Frontend (React)

### 4.1 Estado de sesión

`get_session` devuelve `SessionState | null`:

```ts
type SessionState = {
  user: SessionUser;
  locked: boolean;
  lastActivityAt: string | null;
  idleTimeoutMinutes: number;
};
```

`session-store.ts` guarda `session`, `locked`, `idleTimeoutMinutes` y `lockedReason`; `setSession` acepta el estado completo. La hidratación de `App.tsx` se adapta: si llega `locked`, se pinta la pantalla de desbloqueo en lugar del login.

### 4.2 Autobloqueo con aviso

`src/hooks/use-idle-lock.ts` sustituye a `use-session-timeout.ts`:

- Reinicia el temporizador con actividad real (`mousemove`, `keydown`, `click`, `scroll`, `touchstart`) y al cambiar de vista.
- A `idle_warning_seconds` del límite muestra un aviso persistente: "La sesión se bloqueará en 60 s" con botón **Seguir trabajando** (reinicia el contador).
- Al cumplirse el tiempo llama a `lockSession("inactividad")`; el backend marca la sesión bloqueada y la UI pinta `LockScreen` **sin desmontar el contexto de datos** (se conserva la vista actual en `ui-store`).
- Mientras está bloqueada, el árbol principal no se renderiza, para que las queries montadas dejen de refrescar y no reciban 403 en cascada.

`LockScreen.tsx`:

- Nombre y rol del usuario, campo de contraseña, botón **Desbloquear** (Enter), enlace **Cambiar de usuario** (logout).
- Mensaje con intentos restantes ante contraseña incorrecta y bloqueo de la cuenta al agotarlos.
- Si el backend responde que no hay sesión (proceso reabierto, sesión vencida en otra operación), se cae al login con el aviso correspondiente.

`TopBar` gana un botón **Bloquear** (icono de candado) disponible en todo momento.

### 4.3 Re-autenticación

`src/hooks/use-sensitive-action.ts`:

```ts
const runSensitive = useSensitiveAction();
await runSensitive("VOID_PAYMENT", { title: "Anular pago REC-000123", detail: "40.000 en efectivo" },
  (token) => api.voidPayment(paymentId, reason, token));
```

- Ejecuta la operación con `token` opcional; si el backend responde `REAUTH_REQUIRED`, abre `ReauthDialog` con el título/detalle de la acción.
- El diálogo pide la contraseña del **usuario activo**, muestra qué se va a autorizar y advierte la vigencia del token.
- Con el token, reintenta la operación **una sola vez**; si vuelve a fallar por autorización, no insiste y muestra el error.
- `ReauthDialog` es reutilizable y no conoce la operación concreta: recibe `action`, `title`, `detail`.

Puntos de uso:

- `InvoiceDetailDialog` → anular pago y anular factura.
- `CashPage`/`CashMovementDialog` → gasto, retiro y ajuste.
- El resto de la UI no cambia.

### 4.4 Configuración y usuarios (ADMIN)

- `SettingsPage`: sección **Seguridad** con los cinco valores, descripciones de su efecto y validación de rangos; nota de que la inactividad se aplica desde el siguiente inicio de sesión.
- `UsersPage`: columna de estado ("Bloqueado hasta 20:15") y acción **Desbloquear**; última conexión visible.
- `AuditLogPage`: los nuevos eventos aparecen con el filtro de acción existente, sin cambios.

---

## 5. Fases y estimación

| Fase | Contenido | Estimación |
| --- | --- | --- |
| **1. Bloqueo persistido** | `0043`, contador/umbral/duración en BD, `unlock_user`, auditoría, `list_users` con estado | 2–3 días |
| **2. Sesión bloqueable** | `ActiveSession`, `lock_session`/`unlock_session`, `get_session` con estado, `LockScreen`, `use-idle-lock`, botón Bloquear | 3–4 días |
| **3. Re-autenticación** | `SensitiveAction`, tokens, `authorize_sensitive_action`, `ReauthRequired`, comandos sensibles, `ReauthDialog` | 3 días |
| **4. Configuración y pulido** | Sección Seguridad, columnas de Usuarios, aviso de inactividad, tests de regresión | 2 días |

Total MVP: **~1.5–2 semanas**. Un commit por fase con CI verde.

---

## 6. Criterios de aceptación

### Bloqueo de login

- [ ] Con `max_attempts = 5`, el quinto fallo bloquea al usuario y el mensaje indica el tiempo restante.
- [ ] El bloqueo **sobrevive al reinicio** de la app y se aplica desde otra PC con la misma base.
- [ ] Durante el bloqueo no se verifica la contraseña (no se gasta Argon2) y el mensaje no revela si la contraseña era correcta.
- [ ] Un login correcto antes del umbral reinicia el contador a cero.
- [ ] Al expirar la duración configurada, el usuario puede entrar sin intervención.
- [ ] ADMIN desbloquea desde Usuarios y el usuario entra de inmediato; queda `USER_UNLOCKED` en auditoría.
- [ ] Cada bloqueo deja `LOGIN_LOCKED` con el umbral aplicado; los fallos sueltos siguen dejando `LOGIN_FAILED`.

### Autobloqueo y desbloqueo

- [ ] Sin actividad durante `idle_lock_minutes`, la app muestra la pantalla de bloqueo conservando la vista en la que estaba.
- [ ] `idle_warning_seconds` antes aparece el aviso, y **Seguir trabajando** reinicia el contador.
- [ ] Desbloquear con la contraseña correcta vuelve exactamente a la vista anterior y recarga los datos.
- [ ] Una contraseña incorrecta muestra los intentos restantes; al agotarlos la cuenta se bloquea y se vuelve al login.
- [ ] Con la sesión bloqueada, una llamada directa a un comando (por ejemplo `list_invoices`) devuelve `Forbidden`, aunque la UI no intervenga.
- [ ] Un comando tras superar la inactividad también se rechaza y obliga a desbloquear.
- [ ] **Bloquear** en la barra superior funciona sin esperar inactividad y deja `SESSION_LOCKED`.

### Re-autenticación

- [ ] Anular un pago sin token devuelve `REAUTH_REQUIRED` y **no** modifica nada.
- [ ] El diálogo pide la contraseña, muestra la acción y su importe, y al confirmar la operación se ejecuta una sola vez.
- [ ] El token no se puede reutilizar para otra operación, para otro usuario ni después de `reauth_minutes`.
- [ ] Anular factura y gasto/retiro/ajuste de caja exigen token de su acción correspondiente; un token de pago no sirve para caja.
- [ ] Una contraseña incorrecta en el diálogo no ejecuta la acción, cuenta para el bloqueo y deja `REAUTH_DENIED`.
- [ ] Los tokens no aparecen en la auditoría, en logs ni en el estado persistido.
- [ ] Un usuario sin capacidad de facturación sigue recibiendo `Forbidden` aunque tenga un token válido.

### Configuración y regresión

- [ ] Los cinco valores se editan solo como ADMIN, con validación de rango y defaults si están vacíos.
- [ ] Cambiar la inactividad no afecta a la sesión en curso (se avisa en la UI) y sí al siguiente login.
- [ ] Los flujos actuales de cobro, anulación y caja siguen funcionando (con el paso extra de autorización) y la suite existente pasa.

---

## 7. Pruebas

| Nivel | Casos |
| --- | --- |
| Rust (Firebird real) | Incremento atómico del contador; bloqueo al umbral; login durante bloqueo; expiración; reset por login correcto; desbloqueo ADMIN; `list_users` con estado; dos "PCs" incrementando en paralelo (dos conexiones). |
| Rust (unitario) | Política resuelta con defaults y rangos; token único, no reutilizable, por acción y con expiración; transición de `ActiveSession` (activa → vencida → bloqueada). |
| Frontend (Vitest) | `use-idle-lock` (aviso, reinicio, bloqueo); `use-sensitive-action` (REAUTH_REQUIRED → diálogo → reintento único); `LockScreen` (intentos restantes, cambiar de usuario); store con `locked`. |
| E2E (Playwright) | Login con 5 fallos → bloqueo → desbloqueo ADMIN; inactividad corta configurada → bloqueo → desbloqueo; anular pago pidiendo contraseña. |
| Migración | Base en `0036` → `0043`: columnas nuevas con defaults, ningún usuario bloqueado tras migrar, `CLINIC_SETTINGS` sin claves (usa defaults). |
| Regresión | Cobros, caja, facturas y dashboard con el token añadido; auditar que las firmas nuevas no rompen los comandos existentes. |

Verificación prevista:

```bash
cd src-tauri && cargo test --lib repositories::auth security
npm run check:bindings && npx tsc -b && npm run lint && npm test && npm run build
```

> Nota: `set_invoice_status`, `void_payment` y `add_cash_movement` cambian de firma (parámetro opcional `reauthToken`). Hay que actualizar `src/lib/api.ts`, `src/bindings.ts` (mantenido a mano en este repo), los hooks y las pruebas de frontend en el mismo PR de la fase 3.

---

## 8. Riesgos y mitigaciones

| Riesgo | Impacto | Mitigación |
| --- | --- | --- |
| Contador compartido entre PCs con carreras | Medio (bloqueos falsos) | Incremento atómico en una sola sentencia y bloqueo fijado en la misma transacción |
| Usuarios inexistentes sin contador persistido | Bajo (fuerza bruta) | Throttle en memoria como hoy + Argon2 + mensaje genérico; no hay fila que bloquear |
| Enumeración de usuarios por el mensaje de bloqueo | Bajo/medio | Mensaje genérico para usuario inexistente; el bloqueo solo se anuncia a quien ya conoce el nombre |
| Inactividad demasiado agresiva en cirugías o consultas largas | Medio (fricción) | Valor configurable, aviso previo y "Seguir trabajando"; mínimo 1 minuto |
| Fricción de la re-autenticación en mostrador | Medio (adopción) | Token de 2 min, un solo diálogo por operación y mensajes con la acción concreta |
| El frontend olvida pasar el token y bloquea el cobro | Alto (operación) | Error `REAUTH_REQUIRED` estandarizado + hook único que reintenta solo |
| Cambios de firma en comandos existentes | Medio | Un PR por fase, `check:bindings` y pruebas de frontend en el mismo cambio |
| Política congelada al iniciar sesión | Bajo (confusión) | Nota explícita en Configuración y mensaje al guardar |
| Reloj del servidor vs. cliente | Bajo | Bloqueo y expiración se evalúan en el backend (Firebird), nunca con el reloj del frontend |

---

## 9. Orden de implementación (primer PR)

1. `0043_security.sql` con columnas e índice; registrar en `db/migrations.rs`; probar sobre `0036`.
2. `repositories/auth.rs`: incremento atómico, bloqueo, expiración, reset, `unlock_user`, `list_users` ampliado; tests con Firebird real (incluido el caso de dos conexiones).
3. `commands/auth.rs`: `login` contra el contador de BD, `unlock_user`, auditoría y mensajes; `error.rs` con `ReauthRequired`.
4. `security.rs` + `models/security.rs`: `SecurityPolicy`, `SensitiveAction`, generación y validación de tokens; unit tests.
5. `state.rs`/`auth.rs`: `ActiveSession`, comprobaciones en `require_session`, `lock_session`, `unlock_session`, `get_session` con `SessionState`.
6. Frontend: `session-store`, `LockScreen`, `use-idle-lock`, botón Bloquear, adaptación de `App.tsx`; tests.
7. Fase 3: `authorize_sensitive_action`, token en los tres comandos sensibles, `use-sensitive-action` + `ReauthDialog` y sus puntos de uso.
8. Fase 4: claves `security.*` en `settings.rs` y sección Seguridad; columnas de Usuarios; `cargo test` filtrado, `tsc`, `lint`, `vitest`, `build` y E2E.
