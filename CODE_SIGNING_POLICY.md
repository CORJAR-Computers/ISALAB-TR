# Política de firma de código (Code Signing Policy)

Este proyecto firma y distribuye artefactos de release. El método de firma
difiere según la plataforma.

## Windows — SignPath Foundation (pendiente de aprobación)

Estamos solicitando el ingreso al programa de **SignPath Foundation** para
firmar el instalador de Windows.

Declaración requerida por el programa (si se aprueba):

> **Free code signing provided by SignPath.io, certificate by SignPath Foundation**

Estado: *pendiente de aprobación de la solicitud OSS.*

### Qué se firma

- Instalador Windows NSIS (`ISALAB_<versión>_x64-setup.exe`) publicado en
  GitHub Releases.

### Proceso de build y firma

- Los artefactos se construyen **exclusivamente** desde este repositorio
  público mediante GitHub Actions (workflow `release.yml`).
- Solo los artefactos construidos por CI se envían a SignPath para firmar.
- La clave privada de firma la custodia **SignPath (HSM)**. Este proyecto
  **no almacena la clave privada**.
- El instalador firmado se sustituye en el release y se **re-firma con
  minisign** para el auto-updater (la firma Authenticode modifica el `.exe`
  e invalidaría la firma previa del updater).

### Roles del equipo (proyecto de un único mantenedor)

- **Autores** (acceso de commit, pueden modificar el repositorio sin
  revisiones adicionales):
  - <https://github.com/aleksei-corom>
- **Revisores** (revisión requerida para los cambios propuestos por no
  mantenedores, p. ej. pull requests):
  - <https://github.com/aleksei-corom>
  - Política: todos los pull requests externos son revisados por el
    mantenedor antes del merge.
- **Aprobadores** (aprueban cada solicitud de firma):
  - <https://github.com/aleksei-corom>
  - Política: cada solicitud de firma requiere la **aprobación explícita**
    del mantenedor.

## macOS

No aplica (sin artefactos macOS distribuidos actualmente).

## Linux (sin firmar actualmente)

Estado: no implementado.

### Qué se distribuye

- Por el momento no se distribuyen artefactos Linux.

### Verificación

- Los usuarios deben obtener los artefactos solo desde la página oficial de
  GitHub Releases.

## Ubicaciones de distribución

- <https://github.com/CORJAR-Computers/ISALAB-TR/releases>

## Política de privacidad

Este programa no transfiere información a otros sistemas en red salvo que el
usuario lo configure explícitamente. La clave de API de IA (Groq) se
almacena localmente **cifrada con DPAPI de Windows** y solo se envía al
proveedor de IA cuando el usuario utiliza la función de prediagnóstico.
