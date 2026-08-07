# build-and-sign.ps1
# ============================================================================
# Construye el instalador NSIS local y lo firma para el auto-updater (minisign).
#
# Flujo (mismo que release.yml pero en local):
#   1. Pre-flight: valida la llave, el password y la consistencia de versiones
#      ANTES del build largo (un password erróneo aborta en segundos, no en 30 min).
#   2. `npm run tauri:build` (limpia target + poda Firebird) genera:
#        ISALAB_<version>_x64-setup.exe
#        ISALAB_<version>_x64-setup.exe.sig   <- firma minisign del updater
#   3. Verifica el formato de la firma y genera latest.json (el CLI no lo produce).
#
# SEGURIDAD:
#   - El password NO se acepta por parámetro (quedaría en el historial de
#     PowerShell y visible en la línea de proceso). Se lee de la env var
#     TAURI_SIGNING_PRIVATE_KEY_PASSWORD o se pide con Read-Host -AsSecureString.
#   - El password nunca se imprime ni se escribe a disco.
#   - Se comprueba que la pubkey de la llave local coincide con la embebida en
#     tauri.conf.json: firmar con una llave distinta produciría actualizaciones
#     que el cliente rechazaría (mismatch de firma).
#
# Uso (PowerShell):
#   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "..."; .\scripts\build-and-sign.ps1
#   .\scripts\build-and-sign.ps1 -NoPassword          # llave SIN password (común con --ci)
#   .\scripts\build-and-sign.ps1                       # pide password seguro
#   .\scripts\build-and-sign.ps1 -RetryBuild 1         # reintenta el build 1 vez
#   .\scripts\build-and-sign.ps1 -SkipBuild            # solo re-firma + latest.json
#
# NOTA: esta firma es SOLO para el auto-updater (minisign). La firma
# Authenticode (evitar SmartScreen) se hace con SignPath en CI; si firmas el
# exe por fuera, re-firma el .sig al final (ver release.yml).
# ============================================================================

param(
  [string]$KeyPath = "$HOME\.tauri\isalab.key",
  [int]$RetryBuild = 0,
  [switch]$SkipBuild,
  [switch]$NoPassword
)

$ErrorActionPreference = "Stop"

function Write-Step([string]$msg) {
  Write-Host ""
  Write-Host "==> $msg" -ForegroundColor Cyan
}

if (-not (Get-Command npx -ErrorAction SilentlyContinue)) {
  throw "npx no está disponible en el PATH. Instala Node.js (npm) y reintenta."
}

# ---- 0. Entorno: raíz del repo (paths con espacios OK) ---------------------
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $root

# ---- 1. Pre-flight: llave y password ---------------------------------------
if (-not (Test-Path $KeyPath)) {
  throw "Llave de firma no encontrada en: $KeyPath (¿la generaste con 'npx tauri signer generate -w ~/.tauri/isalab.key'?)"
}
$pubPath = "$HOME\.tauri\isalab.key.pub"
if (-not (Test-Path $pubPath)) {
  throw "Llave pública no encontrada en: $pubPath"
}
$pubKey = (Get-Content $pubPath -Raw).Trim()

# El CLI de Tauri espera el CONTENIDO base64 de la llave, no la ruta.
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -Raw $KeyPath).Trim()

# Password: env var, -NoPassword o prompt seguro (SecureString). NUNCA por
# parámetro. Las llaves generadas con `--ci` (o prompt en blanco) NO tienen
# password: en ese caso usar -NoPassword (env var vacía también es válida).
# Se distingue $null (no configurado → preguntar) de "" (vacío → válido).
if ($NoPassword) {
  $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
} elseif ($null -eq $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
  $secure = Read-Host "Password de la llave de firma (vacío si no tiene)" -AsSecureString
  if ($null -ne $secure -and $secure.Length -gt 0) {
    $ptr = [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    try {
      $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [System.Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr)
    } finally {
      [System.Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
    }
  } else {
    # Prompt en blanco o Enter → llave sin password.
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
  }
}

# ---- 2. Pre-flight: consistencia de versiones ------------------------------
# Tauri nombra el instalador con la versión de tauri.conf.json, pero el script
# y latest.json usan package.json; si divergen, el .exe no se encuentra.
$verPkg  = (Get-Content package.json -Raw | ConvertFrom-Json).version
$verConf = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
$verCargo = ([regex]::Match((Get-Content src-tauri/Cargo.toml -Raw), '(?m)^version\s*=\s*"([^"]+)"')).Groups[1].Value
$versions = @{ "package.json" = $verPkg; "tauri.conf.json" = $verConf; "Cargo.toml" = $verCargo }
$unique = @($versions.Values | Select-Object -Unique)
if ($unique.Count -ne 1) {
  throw "Versiones inconsistentes entre archivos: $($versions | ConvertTo-Json -Compress). Corrige antes de firmar."
}
$version = $unique[0]

# ---- 3. Pre-flight: la llave local coincide con la del cliente -------------
$cfgPub = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).plugins.updater.pubkey
if ($cfgPub -ne $pubKey) {
  throw "La pubkey local ($HOME\.tauri\isalab.key.pub) NO coincide con la embebida en tauri.conf.json. El cliente rechazaría las actualizaciones."
}

# ---- 4. Pre-flight: validar el password firmando un probe (segundos) -------
# Si el password es erróneo, abortamos aquí antes de compilar 10-30 min.
$probeDir = Join-Path $env:TEMP ("isalab-probe-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $probeDir | Out-Null
$probe = Join-Path $probeDir "probe.txt"
Set-Content -Path $probe -Value "isalab-probe-$(Get-Date -Format o)" -Encoding UTF8
try {
  Write-Step "Validando llave y password (firma de prueba)"
  # La llave ya está en TAURI_SIGNING_PRIVATE_KEY (env var): el CLI rechaza
  # -f/--private-key-path cuando la env var está presente, así que solo el path.
  npx tauri signer sign "$probe" 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "Password o llave inválidos. Si la llave NO tiene password usa -NoPassword (o env var vacía)."
  }
  if (-not (Test-Path "$probe.sig")) {
    throw "La firma de prueba no se generó: password o llave inválidos. Prueba con -NoPassword si la llave no tiene password."
  }
  Write-Host "  Llave y password OK (probe firmado correctamente)" -ForegroundColor Green
} finally {
  Remove-Item -Path $probeDir -Recurse -Force -ErrorAction SilentlyContinue
}

# ---- 5. Build ---------------------------------------------------------------
# El target dir puede estar redirigido por .cargo/config.toml local
# (build.target-dir, ej. "D:/rust-targets/isalab"): se detecta para no
# asumir src-tauri/target.
$exe = "ISALAB_${version}_x64-setup.exe"
$targetDir = $null
$cargoConfig = Join-Path $root "src-tauri\.cargo\config.toml"
if (Test-Path $cargoConfig) {
  $cfgText = Get-Content $cargoConfig -Raw
  $m = [regex]::Match($cfgText, '(?m)^\s*target-dir\s*=\s*"([^"]+)"')
  if ($m.Success) { $targetDir = $m.Groups[1].Value }
}
if ([string]::IsNullOrEmpty($targetDir)) { $targetDir = "src-tauri\target" }
if (-not [System.IO.Path]::IsPathRooted($targetDir)) {
  $targetDir = Join-Path $root $targetDir
}
$bundleDir = Join-Path $targetDir "release\bundle\nsis"
$exePath = Join-Path $bundleDir $exe

Write-Step "Firmando updater para v$version con la llave local"
Write-Host "  Llave : $KeyPath"
Write-Host "  Exe   : $exe"
Write-Host "  Build : $(if (-not $SkipBuild) { 'completo (release)' } else { 'omitido (solo re-firma + latest.json)' })"

if (-not $SkipBuild) {
  $attempts = $RetryBuild + 1
  for ($i = 1; $i -le $attempts; $i++) {
    if ($attempts -gt 1) { Write-Step "Build intento $i de $attempts" }
    Write-Step "Ejecutando npm run tauri:build (release, puede tardar 10-30 min)"
    npm run tauri:build
    if ($LASTEXITCODE -eq 0) { break }
    if ($i -lt $attempts) {
      Write-Host "  Build falló (exit $LASTEXITCODE). Reintentando en 10 s..." -ForegroundColor Yellow
      Start-Sleep -Seconds 10
    } else {
      throw "tauri build falló tras $attempts intento(s) (exit $LASTEXITCODE)"
    }
  }
}

if (-not (Test-Path $exePath)) {
  throw "Instalador no encontrado: $exePath"
}
if ((Get-Item $exePath).Length -eq 0) {
  throw "El instalador está vacío (0 bytes): $exePath"
}

# ---- 6. Verificar firma del updater ----------------------------------------
$sigPath = "$exePath.sig"
if (-not (Test-Path $sigPath)) {
  throw "Firma .sig no generada: $sigPath — revisa que createUpdaterArtifacts=true y las env vars de firma"
}
$sigContent = (Get-Content $sigPath -Raw).Trim()
# La firma minisign de Tauri comienza con el comentario base64
# "untrusted comment: signature from tauri secret key" y contiene el sello.
if ($sigContent.Length -lt 100 -or -not $sigContent.StartsWith("dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkK")) {
  throw "La firma $sigPath no tiene el formato esperado de Tauri updater (¿se firmó con la llave correcta?)."
}
Write-Step "Firma del updater OK"
Write-Host "  $sigPath"

# ---- 7. Generar latest.json (manifiesto del auto-updater) ------------------
Write-Step "Generando latest.json"
$url = "https://github.com/CORJAR-Computers/ISALAB-TR/releases/download/v${version}/${exe}"
$manifest = [ordered]@{
  version   = $version
  notes     = "ISALAB v${version}"
  pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = $sigContent
      url       = $url
    }
  }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -Path (Join-Path $bundleDir "latest.json") -Encoding UTF8

# ---- 8. Resumen -------------------------------------------------------------
Write-Step "Listo. Artefactos en $bundleDir"
Get-ChildItem $bundleDir -File | Where-Object { $_.Name -like "ISALAB*" -or $_.Name -eq "latest.json" } |
  ForEach-Object { Write-Host ("  {0,-55} {1,10:N1} MB" -f $_.Name, ($_.Length / 1MB)) }

Write-Host ""
Write-Host "Siguiente paso:" -ForegroundColor Yellow
Write-Host "  - Sube a un release de GitHub (draft) los 3 archivos para activar el auto-updater:"
$exeQ = "'" + $exePath + "'"
$sigQ = "'" + $sigPath + "'"
$latestQ = "'" + (Join-Path $bundleDir "latest.json") + "'"
Write-Host "      gh release create v$version $exeQ $sigQ $latestQ --draft --repo CORJAR-Computers/ISALAB-TR"
Write-Host "  - Si firmas Authenticode (SignPath) por fuera, re-firma el .sig DESPUÉS y regenérate latest.json con:"
Write-Host "      .\scripts\build-and-sign.ps1 -SkipBuild   (usa el password de la env var TAURI_SIGNING_PRIVATE_KEY_PASSWORD)"
