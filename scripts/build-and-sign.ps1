# build-and-sign.ps1
# ============================================================================
# Construye el instalador NSIS local y lo firma para el auto-updater (minisign).
#
# Flujo (mismo que release.yml pero en local):
#   1. Limpia target + poda Firebird a subset embedded (npm run tauri:build)
#   2. `tauri build` con createUpdaterArtifacts genera:
#        ISALAB_<version>_x64-setup.exe
#        ISALAB_<version>_x64-setup.exe.sig   <- firma minisign del updater
#   3. Genera latest.json (el CLI NO lo produce; lo hace el workflow en CI)
#
# Requisitos:
#   - Llave privada del updater en ~/.tauri/isalab.key (o -KeyPath)
#   - El password de la llave: se pide interactivamente o se pasa con -Password
#     (también acepta la env var TAURI_SIGNING_PRIVATE_KEY_PASSWORD).
#
# Uso (PowerShell):
#   .\scripts\build-and-sign.ps1                      # pide password
#   .\scripts\build-and-sign.ps1 -Password "..."      # sin prompt
#
# NOTA: esta firma es SOLO para el auto-updater (minisign). La firma
# Authenticode (evitar SmartScreen) se hace con SignPath en CI; si firmas el
# exe por fuera, re-firma el .sig al final (ver release.yml).
# ============================================================================

param(
  [string]$KeyPath = "$HOME\.tauri\isalab.key",
  [string]$Password = ""
)

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

function Write-Step([string]$msg) {
  Write-Host ""
  Write-Host "==> $msg" -ForegroundColor Cyan
}

# ---- 1. Llave de firma ------------------------------------------------------
if (-not (Test-Path $KeyPath)) {
  throw "Llave de firma no encontrada en: $KeyPath (¿la generaste con 'npx tauri signer generate -w ~/.tauri/isalab.key'?)"
}

# El CLI de Tauri espera el CONTENIDO base64 de la llave, no la ruta.
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -Raw $KeyPath).Trim()

if ([string]::IsNullOrEmpty($Password)) {
  $Password = $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
}
if ([string]::IsNullOrEmpty($Password)) {
  $Password = Read-Host "Password de la llave de firma (isalab.key)"
}
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $Password

$version = (Get-Content package.json | ConvertFrom-Json).version
$exe = "ISALAB_${version}_x64-setup.exe"
$bundleDir = "src-tauri\target\release\bundle\nsis"
$exePath = Join-Path $bundleDir $exe

Write-Step "Firmando updater para v$version con la llave local"
Write-Host "  Llave : $KeyPath"
Write-Host "  Exe   : $exe"

# ---- 2. Build (limpia target, poda Firebird, compila y empaqueta) ----------
Write-Step "Ejecutando npm run tauri:build (release, puede tardar 10-30 min)"
npm run tauri:build
if ($LASTEXITCODE -ne 0) { throw "tauri build falló (exit $LASTEXITCODE)" }

if (-not (Test-Path $exePath)) {
  throw "Instalador no encontrado: $exePath"
}

# ---- 3. Verificar firma del updater ----------------------------------------
$sigPath = "$exePath.sig"
if (-not (Test-Path $sigPath)) {
  throw "Firma .sig no generada: $sigPath — revisa que createUpdaterArtifacts=true y las env vars de firma"
}
Write-Step "Firma del updater OK"
Write-Host "  $sigPath"

# ---- 4. Generar latest.json (manifiesto del auto-updater) ------------------
Write-Step "Generando latest.json"
$signature = (Get-Content $sigPath -Raw).Trim()
$repo = "CORJAR-Computers/ISALAB-TR"
$url = "https://github.com/$repo/releases/download/v${version}/${exe}"
$manifest = [ordered]@{
  version   = $version
  notes     = "ISALAB v${version}"
  pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = $signature
      url       = $url
    }
  }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -Path (Join-Path $bundleDir "latest.json") -Encoding UTF8

# ---- 5. Resumen -------------------------------------------------------------
Write-Step "Listo. Artefactos en $bundleDir"
Get-ChildItem $bundleDir -File | Where-Object { $_.Name -like "ISALAB*" -or $_.Name -eq "latest.json" } |
  ForEach-Object { Write-Host ("  {0,-55} {1,10:N1} MB" -f $_.Name, ($_.Length / 1MB)) }

Write-Host ""
Write-Host "Siguiente paso:" -ForegroundColor Yellow
Write-Host "  - Sube a un release de GitHub (draft) los 3 archivos para activar el auto-updater:"
Write-Host "      gh release create v$version $exePath `"$sigPath`" `"$bundleDir\latest.json`" --draft --repo $repo"
Write-Host "  - Si vas a firmar Authenticode (SignPath), hazlo ANTES de subir y re-firma el .sig:"
Write-Host "      npx tauri signer sign -f `"$KeyPath`" -p `"$Password`" `"$exePath`""
