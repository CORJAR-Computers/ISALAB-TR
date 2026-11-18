# Servidor Firebird 5 para ISALAB

Configuración de referencia del servidor central de la clínica. Se aplica una
sola vez, en la máquina que hace de servidor (Windows 11 Tiny).

## Archivos

| Archivo | Destino en el servidor |
| --- | --- |
| `firebird.conf.snippet` | Añadir al final de `<Firebird>\firebird.conf` |
| `databases.conf.example` | Reemplaza el bloque de bases vivas de `<Firebird>\databases.conf` |

`<Firebird>` es el directorio de instalación (`C:\Firebird\Firebird_5_0` con el
instalador, o la carpeta que se haya descomprimido con el ZIP).

## Qué se configura

- **Alias `ISALAB`** → `C:\Firebird\data\isalab.fdb`. Los clientes conectan a
  `192.168.1.10/3050:ISALAB`; la ruta real nunca sale del servidor.
- **`RemoteAuxPort = 3051`**: puerto fijo de eventos. Sin él, el servidor usa un
  puerto aleatorio en cada arranque, el firewall lo bloquea y los clientes no
  reciben notificaciones.
- **`DatabaseAccess = Restrict …`**: solo se abren bases del directorio de datos
  y la base de seguridad. El separador es `;`, no la coma.
- **`RemoteAccess = false`** en la base de seguridad: nadie se conecta a
  `security5.fdb` desde la red.

## Permisos de carpeta

La cuenta del servicio (por defecto `LocalSystem`) necesita lectura y escritura
en `C:\Firebird\data` y `C:\Firebird\backups`. Si el servicio corre con una
cuenta dedicada (`instsvc install -login`), hay que dárselos explícitamente.

## Pasos

```powershell
# 1. Detener el servicio antes de editar la configuración.
Restart-Service FirebirdServerDefaultInstance -Force   # (ejecutar tras editar)

# 2. Comprobar que arranca.
Get-Service FirebirdServerDefaultInstance

# 3. Verificar desde otra PC de la clínica.
.\scripts\verify-firebird-remote.ps1 -Server 192.168.1.10
```

El procedimiento completo, incluido backup/restore y actualizaciones, está en
`docs/firebird-migration-runbook.md`.
