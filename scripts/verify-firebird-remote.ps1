<#
.SYNOPSIS
    Comprueba que el servidor Firebird de ISALAB es alcanzable desde esta PC.

.DESCRIPTION
    Verifica, en este orden:
      1. Que el puerto del servicio (3050 por defecto) acepta conexiones.
      2. Que el puerto de eventos remotos (3051) está abierto.
      3. Que un usuario real autentica contra el alias de la base.
      4. La versión del esquema (si el runner ya se ejecutó).

    Pensado para ejecutarse desde una PC de la clínica, no desde el servidor:
    valida exactamente lo que valida la aplicación (red + firewall + alias +
    credenciales + esquema).

.PARAMETER Server
    IP o nombre del servidor. Defecto: 192.168.1.10.

.PARAMETER Port
    Puerto del servicio. Defecto: 3050.

.PARAMETER AuxPort
    Puerto de eventos remotos (RemoteAuxPort). Defecto: 3051.

.PARAMETER Database
    Alias o ruta de la base. Defecto: ISALAB.

.PARAMETER User
    Usuario con el que probar. Defecto: ISALAB_APP.

.PARAMETER Password
    Contraseña. Si se omite se toma de ISALAB_DB_PASSWORD o se pide por consola.
    No se recomienda pasarla por línea de comandos: queda en el historial.

.PARAMETER IsqlPath
    Ruta a isql.exe. Si se omite se busca en el repo, en FIREBIRD_DIR, en las
    rutas habituales de instalación y en el PATH.

.PARAMETER SkipAuxPort
    Omite la comprobación del puerto de eventos (útil si el firewall aún no lo
    permite pero la app funciona sin invalidación en tiempo real).

.EXAMPLE
    .\verify-firebird-remote.ps1 -Server 192.168.1.10

.EXAMPLE
    $env:ISALAB_DB_PASSWORD = 'secreto'
    .\verify-firebird-remote.ps1 -Server 192.168.1.10 -User SYSDBA
#>
[CmdletBinding()]
param(
    [string]$Server = '192.168.1.10',
    [int]$Port = 3050,
    [int]$AuxPort = 3051,
    [string]$Database = 'ISALAB',
    [string]$User = 'ISALAB_APP',
    [string]$Password = $env:ISALAB_DB_PASSWORD,
    [string]$IsqlPath,
    [switch]$SkipAuxPort
)

$ErrorActionPreference = 'Stop'
$script:Failures = 0

function Write-Result {
    param([bool]$Ok, [string]$Message, [string]$Detail)
    if ($Ok) {
        Write-Host ('  [OK]   ' + $Message) -ForegroundColor Green
    } else {
        Write-Host ('  [FALLO] ' + $Message) -ForegroundColor Red
        $script:Failures++
    }
    if ($Detail) {
        Write-Host ('         ' + $Detail) -ForegroundColor DarkGray
    }
}

function Write-Warn {
    param([string]$Message, [string]$Detail)
    Write-Host ('  [AVISO] ' + $Message) -ForegroundColor Yellow
    if ($Detail) {
        Write-Host ('         ' + $Detail) -ForegroundColor DarkGray
    }
}

function Test-TcpPort {
    param([string]$TargetHost, [int]$TargetPort, [int]$TimeoutMs = 5000)
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $task = $client.ConnectAsync($TargetHost, $TargetPort)
        if (-not $task.Wait($TimeoutMs)) { return $false }
        return $client.Connected
    } catch {
        return $false
    } finally {
        $client.Dispose()
    }
}

function Resolve-Isql {
    param([string]$Explicit)
    if ($Explicit) {
        if (Test-Path -LiteralPath $Explicit) { return (Resolve-Path -LiteralPath $Explicit).Path }
        return $null
    }
    $candidates = @(
        (Join-Path $PSScriptRoot '..\src-tauri\binaries\firebird\isql.exe'),
        (Join-Path $PSScriptRoot '..\Firebird-5.0.3.1683-0-windows-x64\isql.exe')
    )
    if ($env:FIREBIRD_DIR) { $candidates += (Join-Path $env:FIREBIRD_DIR 'isql.exe') }
    $candidates += @(
        'C:\Firebird\Firebird_5_0\isql.exe',
        'C:\Firebird\Firebird_5_0_0\isql.exe',
        'D:\Firebird-5.0.3.1683-0\isql.exe'
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    $command = Get-Command isql.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    return $null
}

function Invoke-Isql {
    param([string]$Exe, [string]$Sql)
    $tmp = [System.IO.Path]::GetTempFileName()
    # Las credenciales van por variable de entorno (ISC_USER/ISC_PASSWORD) para
    # que no aparezcan en la línea de comandos del proceso.
    $previousUser = $env:ISC_USER
    $previousPassword = $env:ISC_PASSWORD
    try {
        Set-Content -LiteralPath $tmp -Value $Sql -Encoding ascii
        $env:ISC_USER = $User
        $env:ISC_PASSWORD = $Password
        $output = Get-Content -LiteralPath $tmp |
            & $Exe -q "${Server}/${Port}:${Database}" 2>&1
        return [pscustomobject]@{
            Output   = ($output -join "`n")
            ExitCode = $LASTEXITCODE
        }
    } finally {
        $env:ISC_USER = $previousUser
        $env:ISC_PASSWORD = $previousPassword
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
    }
}

Write-Host ''
Write-Host "Verificación de Firebird para ISALAB" -ForegroundColor Cyan
Write-Host "  Servidor: ${Server}:${Port}"
Write-Host "  Base:     ${Database}"
Write-Host "  Usuario:  ${User}"
Write-Host ''

if (-not $Password) {
    $secure = Read-Host -Prompt "Contraseña de $User" -AsSecureString
    $Password = [System.Net.NetworkCredential]::new('', $secure).Password
}

# 1. Puerto del servicio
Write-Host 'Red' -ForegroundColor Cyan
Write-Result (Test-TcpPort -TargetHost $Server -TargetPort $Port) `
    "TCP ${Port} alcanzable" `
    'Si falla: servicio detenido, IP incorrecta o regla de firewall ausente.'

# 2. Puerto de eventos
#
# No se trata como fallo: Firebird crea el listener del puerto auxiliar cuando
# un cliente se suscribe a eventos, así que con el servidor inactivo puede
# estar cerrado sin que nada esté mal.
if (-not $SkipAuxPort) {
    if (Test-TcpPort -TargetHost $Server -TargetPort $AuxPort) {
        Write-Result $true "TCP ${AuxPort} (eventos remotos) alcanzable" `
            'Los clientes recibirán notificaciones en vivo.'
    } else {
        Write-Warn "TCP ${AuxPort} (eventos remotos) no acepta conexiones ahora mismo" `
            'Es normal si ningún cliente está suscrito todavía. Confirme RemoteAuxPort = 3051 y la regla de firewall; la prueba definitiva es que la UI se refresque al cambiar datos en otro PC.'
    }
}

# 3. Autenticación y alias
Write-Host ''
Write-Host 'Base de datos' -ForegroundColor Cyan

$isql = Resolve-Isql -Explicit $IsqlPath
if (-not $isql) {
    Write-Result $false 'isql.exe no encontrado' `
        'Indique -IsqlPath o instale el cliente Firebird. Sin isql no se puede validar la autenticación.'
} else {
    Write-Host "  isql:     $isql" -ForegroundColor DarkGray

    $probe = Invoke-Isql -Exe $isql -Sql @'
SET LIST ON;
SELECT 'ISALAB_VERIFY_OK' AS MARKER FROM RDB$DATABASE;
SELECT CURRENT_USER AS AUTHENTICATED_USER FROM RDB$DATABASE;
'@

    $authenticated = $probe.Output -match 'ISALAB_VERIFY_OK'
    Write-Result $authenticated "Autenticación contra ${Database}" `
        ($(if ($authenticated) { 'Alias, firewall y credenciales correctos.' }
           else { 'Revise el alias en databases.conf, DatabaseAccess y la contraseña.' }))

    if (-not $authenticated) {
        Write-Host ''
        Write-Host 'Salida de isql:' -ForegroundColor DarkGray
        Write-Host $probe.Output -ForegroundColor DarkGray
    } else {
        # 4. Versión del esquema (puede no existir todavía)
        $version = Invoke-Isql -Exe $isql -Sql @'
SET LIST ON;
SELECT MAX(VERSION) AS SCHEMA_VERSION FROM SCHEMA_MIGRATIONS WHERE STATUS = 'DONE';
'@
        $value = 'desconocida'
        if ($version.Output -match 'SCHEMA_VERSION\s+(\d+)') {
            $value = $Matches[1]
        } elseif ($version.Output -match 'Table unknown|Dynamic SQL Error') {
            $value = '0 (el runner aún no se ha ejecutado)'
        }
        Write-Result $true "Esquema: v$value" `
            'La app espera la versión que trae compilada; si es menor, hay que ejecutar isalab-migrate.'
    }
}

Write-Host ''
if ($script:Failures -eq 0) {
    Write-Host 'Resultado: todo correcto. Las PCs de la clínica pueden conectar.' -ForegroundColor Green
    exit 0
}

Write-Host "Resultado: $script:Failures comprobación(es) fallida(s)." -ForegroundColor Red
Write-Host 'Guía de diagnóstico: docs/firebird-migration-runbook.md' -ForegroundColor Yellow
exit 1
