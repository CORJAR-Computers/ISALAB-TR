# Runbook — Servidor Firebird 5 de ISALAB

Procedimientos operativos del servidor central de la clínica. Todo lo que sigue
está verificado contra Firebird 5.0.3 en Windows 11.

## Topología

```text
   PC consulta 1 ─┐
   PC consulta 2 ─┼─ TCP 3050/3051 ─▶  Servidor (Windows 11 Tiny)
   PC consulta 3 ─┘                     ├─ Firebird 5 (servicio)
                                        ├─ C:\Firebird\data\isalab.fdb
                                        └─ C:\Firebird\backups\*.fbk
```

- La base **solo** vive en el servidor. Nunca se comparte por SMB.
- Los clientes son clientes puros: no crean la base, no migran y no siembran
  datos. Si el esquema no coincide con el que esperan, se niegan a operar.
- El único proceso que modifica el esquema es `isalab-migrate`, y se ejecuta a
  mano una vez por actualización.

## 1. Instalación del servidor

1. Instalar Firebird 5 x64 (instalador o ZIP) en `C:\Firebird\Firebird_5_0`.
2. Registrar el servicio (el ZIP no lo registra solo):

   ```bat
   cd /d "C:\Firebird\Firebird_5_0"
   install_service.bat
   ```

   Comprobar con `Get-Service FirebirdServerDefaultInstance`.
3. Crear las carpetas de datos y copias:

   ```powershell
   New-Item -ItemType Directory -Force C:\Firebird\data, C:\Firebird\backups
   ```

4. Editar `C:\Firebird\Firebird_5_0\firebird.conf` con el contenido de
   `deploy/firebird/firebird.conf.snippet`:

   ```ini
   RemoteAuxPort = 3051
   DatabaseAccess = Restrict C:\Firebird\data;C:\Firebird\Firebird_5_0\security5.fdb
   ```

   > El separador de `DatabaseAccess` es **`;`**, no la coma. Con comas el
   > servidor considera un único directorio inexistente y **toda** conexión
   > falla con `Use of database at location … is not allowed by server
   > configuration`.

5. Editar `C:\Firebird\Firebird_5_0\databases.conf` con
   `deploy/firebird/databases.conf.example` (alias `ISALAB` y `RemoteAccess =
   false` en la base de seguridad).
6. Reiniciar el servicio y abrir el firewall **solo para la subred de la
   clínica**:

   ```powershell
   Restart-Service FirebirdServerDefaultInstance -Force
   New-NetFirewallRule -DisplayName "ISALAB Firebird 3050" -Direction Inbound `
     -Protocol TCP -LocalPort 3050 -Action Allow -RemoteAddress LocalSubnet -Profile Private
   New-NetFirewallRule -DisplayName "ISALAB Firebird 3051" -Direction Inbound `
     -Protocol TCP -LocalPort 3051 -Action Allow -RemoteAddress LocalSubnet -Profile Private
   ```

7. Reservar la IP del servidor en el router (o IP fija) y anotarla. Todos los
   ejemplos usan `192.168.1.10`.

## 2. Base limpia y usuario de aplicación

Una sola vez, como `SYSDBA`, desde el propio servidor:

```sql
CREATE DATABASE 'localhost/3050:C:\Firebird\data\isalab.fdb'
  PAGE_SIZE 16384
  DEFAULT CHARACTER SET UTF8
  USER 'SYSDBA' PASSWORD '<clave-sysdba>';

CREATE USER ISALAB_APP PASSWORD '<clave-app>' USING PLUGIN Srp;
COMMIT;
```

Comprobaciones:

```sql
-- ODS 13.1 y UTF8
SELECT RDB$CHARACTER_SET_NAME FROM RDB$DATABASE;
```

```bat
:: Page size y ODS desde el servidor
gstat -h localhost/3050:ISALAB
```

> Crear la base con otro `PAGE_SIZE` o charset obliga a recrearla: el runner y
> la app asumen UTF8. No copiar nunca un `.fdb` desde una PC de desarrollo; la
> clínica arranca de cero.

## 3. Migrar el esquema (una vez por actualización)

`isalab-migrate` acompaña al instalador. Se ejecuta desde el servidor (o desde
una PC de administración) con una cuenta que tenga permisos de DDL (`SYSDBA`):

```powershell
$env:ISALAB_DB_PASSWORD = '<clave-sysdba>'

# Inspeccionar sin modificar nada
.\isalab-migrate.exe --host 192.168.1.10 --status

# Aplicar migraciones + contraseña inicial de admin + grants
.\isalab-migrate.exe --host 192.168.1.10 --app-user ISALAB_APP --no-demo
```

Qué hace, en orden:

1. Aplica las migraciones pendientes. Cada una se registra primero como
   `RUNNING` y pasa a `DONE` al terminar todas sus sentencias.
2. Fija la contraseña inicial de `admin` (`admin123`, con cambio obligatorio)
   si aún no tiene una.
3. Vuelve a otorgar los permisos a `ISALAB_APP` sobre **todos** los objetos
   (tablas, vistas, generadores, procedimientos y funciones).
4. Poda `EVENT_LOG` con más de 30 días.

Salida esperada la primera vez:

```text
Esquema actual: v0 (esperado v33)
Aplicando migraciones pendientes…
Esquema actualizado a v33.
Contraseña inicial de «admin» establecida (cambio obligatorio al entrar).
Permisos otorgados a «ISALAB_APP»: 48 tablas, 0 vistas, 46 generadores, 2 procedimientos.
Poda de EVENT_LOG: 0 filas antiguas eliminadas.
```

> `--no-demo` es obligatorio en producción: garantiza que la migración de datos
> ficticios (`0005_test_data`) se registre sin insertar nada. `--seed-demo`
> solo se acepta contra `localhost` y nunca debe usarse en la base real.

### Si una migración falla

El runner se detiene y la migración queda en `RUNNING`. La app sigue viendo la
versión anterior y **no** opera. Pasos:

1. Leer el error: incluye la migración, la sentencia y el SQL exacto.
2. Corregir el esquema a mano con `isql` (o restaurar el backup previo).
3. Marcar la migración como aplicada y reintentar:

   ```sql
   UPDATE SCHEMA_MIGRATIONS SET STATUS = 'DONE' WHERE VERSION = <n>;
   COMMIT;
   ```

## 4. Configurar cada PC cliente

1. Instalar la app. El primer arranque muestra el aviso de base de datos no
   configurada.
2. Abrir **Servidor de base de datos** desde la pantalla de login e introducir:
   - Host: `192.168.1.10`
   - Puerto: `3050`
   - Base: `ISALAB`
   - Usuario: `ISALAB_APP`
   - Contraseña: la de la aplicación
3. Probar la conexión y guardar. La contraseña se guarda cifrada con DPAPI en
   `%APPDATA%\db-connection.json`, así que **cada PC y cada usuario de Windows**
   la introduce una vez.

Alternativa por variables de entorno (útil para soporte):
`ISALAB_DB_HOST`, `ISALAB_DB_PORT`, `ISALAB_DB_NAME`, `ISALAB_DB_USER`,
`ISALAB_DB_PASSWORD`.

## 5. Verificación desde un cliente

```powershell
$env:ISALAB_DB_PASSWORD = '<clave-app>'
.\scripts\verify-firebird-remote.ps1 -Server 192.168.1.10
```

Comprueba puerto 3050, puerto 3051, autenticación contra el alias y versión del
esquema. Termina con código 0 si todo está correcto.

> El puerto **3051 puede aparecer cerrado** con el servidor en reposo: Firebird
> crea el listener auxiliar cuando algún cliente se suscribe a eventos. Por eso
> el script lo trata como aviso y no como fallo. La prueba definitiva es abrir
> la app en dos PCs y comprobar que la lista se refresca al guardar en la otra.

Equivalente manual:

```bat
isql -user ISALAB_APP -password <clave-app> 192.168.1.10/3050:ISALAB
SQL> SELECT COUNT(*) FROM PATIENTS;
```

## 6. Backup

### Diario (Programador de tareas de Windows)

Crear `C:\Firebird\backup-isalab.ps1`:

```powershell
$stamp = Get-Date -Format 'yyyyMMdd-HHmm'
$file  = "C:\Firebird\backups\isalab-$stamp.fbk"
& C:\Firebird\Firebird_5_0\gbak.exe -b -t -v `
    -user SYSDBA -password '<clave-sysdba>' `
    localhost/3050:ISALAB $file
if ($LASTEXITCODE -ne 0) { throw "gbak falló con código $LASTEXITCODE" }

# Retención de 30 días
Get-ChildItem C:\Firebird\backups\isalab-*.fbk |
    Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-30) } |
    Remove-Item -Force
```

Programarlo a diario fuera del horario de consulta:

```powershell
$a = New-ScheduledTaskAction -Execute 'powershell.exe' `
     -Argument '-NoProfile -ExecutionPolicy Bypass -File C:\Firebird\backup-isalab.ps1'
$t = New-ScheduledTaskTrigger -Daily -At 22:00
Register-ScheduledTask -TaskName 'ISALAB Backup Firebird' -Action $a -Trigger $t `
     -User 'SYSTEM' -RunLevel Highest
```

> Copiar además los `.fbk` a otro disco o NAS. Un backup en el mismo disco que
> la base no protege de un fallo de disco.

### Restaurar (ensayo o recuperación real)

```bat
:: 1. Si la base está en uso, desconectar a todos
gfix -shut -force 60 localhost/3050:ISALAB

:: 2. Restaurar sobre un archivo nuevo (nunca encima del que está abierto)
gbak -c -r -v C:\Firebird\backups\isalab-20261118-2200.fbk ^
     C:\Firebird\data\isalab_restore.fdb

:: 3. Validar
gfix -validate C:\Firebird\data\isalab_restore.fdb

:: 4. Sustituir: parar el servicio, renombrar el .fdb dañado y poner el restaurado
Restart-Service FirebirdServerDefaultInstance -Force
```

> Nunca copiar un `.fdb` en caliente con el Explorador: la copia queda
> inconsistente. Siempre `gbak -b` / `gbak -c -r`.
>
> Tras restaurar, el `SECURITY5.FDB` del servidor no cambia: los usuarios son
> globales al servidor, no van dentro del backup de la base.

## 7. Actualización de la aplicación

```text
1. Avisar y cerrar las 3 PCs (o aceptar que muestren «desactualizada»).
2. Backup manual:  gbak -b -t localhost/3050:ISALAB C:\Firebird\backups\pre-upgrade.fbk
3. Ejecutar el runner nuevo:  isalab-migrate --host 192.168.1.10 --no-demo
4. Instalar la app nueva en las 3 PCs.
5. Verificar:  scripts\verify-firebird-remote.ps1
```

La app nueva se niega a operar mientras el esquema sea menor que el esperado,
así que el orden inverso (clientes antes que runner) solo produce un aviso
claro, nunca datos corruptos.

## 8. Diagnóstico

| Síntoma | Causa probable | Solución |
| --- | --- | --- |
| `Use of database at location … is not allowed by server configuration` | `DatabaseAccess` mal escrito (comas en vez de `;`) o ruta fuera del directorio permitido | Corregir `firebird.conf` y reiniciar el servicio |
| `Unable to complete network request to host` / `connection rejected by remote interface` | Servicio detenido, IP incorrecta o firewall | `Get-Service`, `Test-NetConnection -Port 3050`, revisar reglas |
| `Your user name and password are not defined` | Usuario inexistente, contraseña errónea o plugin distinto de `Srp` | `CREATE USER … USING PLUGIN Srp` como SYSDBA |
| `no permission for select access to TABLE X` | Falta el grant (objeto nuevo o runner no ejecutado) | Volver a ejecutar `isalab-migrate` |
| La app muestra «Base de datos desactualizada» | Runner pendiente | `isalab-migrate --status` y luego sin `--status` |
| El runner dice que una migración «quedó a medias» | Falló a mitad de una migración | Revisar el esquema y marcarla `DONE` (sección 3) |
| La UI no se refresca cuando otro PC cambia datos | `RemoteAuxPort` sin fijar o puerto 3051 bloqueado | `RemoteAuxPort = 3051` + regla de firewall |
| `database is shutdown` / conflicto de lock | Restore o `gfix` en curso | Esperar o repetir la operación |
| `Install incomplete. To complete security database initialization please CREATE USER` | Base de seguridad sin inicializar (típico en cliente embedded de tests) | En el servidor real no ocurre; en tests, usar una base con usuario creado |

## 9. Rollback

- **Solo app**: reinstalar la versión anterior. Si el esquema ya avanzó, la app
  antigua verá `schema > esperado` y avisará de app desactualizada; hay que
  restaurar el backup previo para volver atrás de verdad.
- **Esquema**: el runner no borra migraciones. Para revertir:

  ```text
  1. gfix -shut -force 60 localhost/3050:ISALAB
  2. Parar el servicio Firebird.
  3. Renombrar C:\Firebird\data\isalab.fdb a isalab.fdb.dañado
  4. gbak -c -r C:\Firebird\backups\pre-upgrade.fbk C:\Firebird\data\isalab.fdb
  5. Arrancar el servicio y verificar.
  ```
