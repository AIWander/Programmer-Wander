<#
.SYNOPSIS
  Build the Programmer Claude Desktop extension (.mcpb) from this repository.

.DESCRIPTION
  Builds programmer.exe with cargo for the host architecture (and x64 with -X64), checks
  each exe (PE machine, MCP initialize + tools/list, tool names equal manifest.json), stages
  the bundle (manifest, icon, docs, LICENSE, NOTICE, server\programmer.exe), validates and
  packs it with the mcpb CLI, unpacks the result to a temp folder and repeats the handshake
  against the unpacked copy, then writes dist\SHA256SUMS.txt and build\pack-report.json.

  It never installs anything and never touches Claude Desktop or its config.

  Requirements: Rust (cargo + the msvc target for each arch), Node.js 20+, and the mcpb CLI
  (npm install -g @anthropic-ai/mcpb).

.PARAMETER X64
  Also build and pack the x86_64 bundle (cross-build when the host is ARM64).
.PARAMETER Arm64
  Also build and pack the ARM64 bundle (cross-build when the host is x64).
.PARAMETER TargetDir
  CARGO_TARGET_DIR to use. Default: mcpb\build\target. Point it at an existing target dir to
  reuse a warm build cache.
.PARAMETER OutDir
  Where the .mcpb files and SHA256SUMS.txt go. Default: mcpb\dist.
.PARAMETER Clean
  Delete OutDir and the staging folder before building.
.PARAMETER Sign
  Authenticode-sign each programmer.exe BEFORE it is packed, by running SignScript on it,
  then (if McpbCert and McpbKey are given) sign each .mcpb with `mcpb sign`.
.PARAMETER SignScript
  Script that signs one file given -Path. Default: C:\CPC\scripts\sign-cpc.ps1 (Azure Trusted
  Signing, account AIWander, profile cpc-signing). See SIGNING.md.
.PARAMETER McpbCert
  PEM certificate for `mcpb sign`. Optional; without it the .mcpb files stay unsigned.
.PARAMETER McpbKey
  PEM private key for `mcpb sign`.
.PARAMETER McpbIntermediate
  Optional PEM intermediate certificate(s) for `mcpb sign`.
#>
[CmdletBinding()]
param(
  [switch]$X64,
  [switch]$Arm64,
  [string]$TargetDir,
  [string]$OutDir,
  [switch]$Clean,
  [switch]$Sign,
  [string]$SignScript = 'C:\CPC\scripts\sign-cpc.ps1',
  [string]$McpbCert,
  [string]$McpbKey,
  [string[]]$McpbIntermediate
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$Here      = $PSScriptRoot
$Repo      = Split-Path -Parent $Here
$Build     = Join-Path $Here 'build'
$Stage     = Join-Path $Build 'stage'
if (-not $OutDir)    { $OutDir = Join-Path $Here 'dist' }
if (-not $TargetDir) { $TargetDir = Join-Path $Build 'target' }
$Handshake = Join-Path $Here 'mcp-handshake.mjs'
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$Arches = [ordered]@{
  'arm64' = @{ triple = 'aarch64-pc-windows-msvc'; machine = 0xAA64 }
  'x64'   = @{ triple = 'x86_64-pc-windows-msvc';  machine = 0x8664 }
}
$BundleFiles = @('manifest.json', 'icon.png', 'README.md', 'PRIVACY.md', 'CHANGELOG.md')
$RepoFiles   = @('LICENSE', 'NOTICE')

$report = [ordered]@{
  started_at = (Get-Date).ToString('o'); host_arch = $env:PROCESSOR_ARCHITECTURE
  repo = $Repo; head = $null; dirty = $null; version = $null; signed_exe = [bool]$Sign
  builds = [ordered]@{}; bundles = [ordered]@{}; warnings = @()
}
function Step([string]$m) { Write-Host ''; Write-Host "== $m" -ForegroundColor Cyan }
function Warn([string]$m) { Write-Warning $m; $script:report.warnings += $m }
function Save-Report {
  New-Item -ItemType Directory -Force -Path $Build | Out-Null
  $report.finished_at = (Get-Date).ToString('o')
  [IO.File]::WriteAllText((Join-Path $Build 'pack-report.json'), ($report | ConvertTo-Json -Depth 8), $Utf8NoBom)
}
function Die([string]$m) { [Console]::Error.WriteLine("ERROR: $m"); Save-Report; exit 1 }
function Invoke-Native([string]$exe, [string[]]$argList, [string]$log) {
  $prev = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
  try { & $exe @argList *> $log } finally { $ErrorActionPreference = $prev }
  return $LASTEXITCODE
}
function Get-Sha([string]$p) { (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() }
function Get-PeMachine([string]$p) {
  $b = [IO.File]::ReadAllBytes($p); $pe = [BitConverter]::ToInt32($b, 0x3C)
  return [BitConverter]::ToUInt16($b, $pe + 4)
}
function Invoke-Handshake([string[]]$cmd, [string[]]$envPairs) {
  $a = @($Handshake, '--timeout', '15000')
  foreach ($e in $envPairs) { $a += @('--env', $e) }
  $a += '--'; $a += $cmd
  $prev = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
  try { $out = & node @a 2>$null } finally { $ErrorActionPreference = $prev }
  return (($out -join "`n") | ConvertFrom-Json)
}
function Assert-ToolsMatch([string]$manifestPath, $live, [string]$label) {
  $m = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
  $diff = @(Compare-Object @($m.tools.name | Sort-Object) @($live.tools.name | Sort-Object))
  if ($diff.Count -gt 0) {
    Die "$label manifest tools differ from live tools/list: $(($diff | ForEach-Object { "$($_.SideIndicator)$($_.InputObject)" }) -join ', ')"
  }
}

# --- 1. tooling and version -------------------------------------------------------------------
Step '1. tooling'
foreach ($t in 'cargo', 'node', 'mcpb') {
  if (-not (Get-Command $t -ErrorAction SilentlyContinue)) {
    if ($t -eq 'mcpb') { Die 'mcpb CLI missing: npm install -g @anthropic-ai/mcpb' }
    Die "$t not on PATH"
  }
}
$report.tooling = [ordered]@{ node = (& node -v); cargo = (& cargo --version); mcpb = (& mcpb --version) }
$report.head = (& git -C $Repo rev-parse --short HEAD)
$report.dirty = [bool](& git -C $Repo status --porcelain -- src Cargo.toml Cargo.lock build.rs)
if ($report.dirty) { Warn 'Rust sources have uncommitted changes; the bundle will not match any commit.' }
$manifest = Get-Content -LiteralPath (Join-Path $Here 'manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$cargoVer = (Select-String -LiteralPath (Join-Path $Repo 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
if ($cargoVer -ne $manifest.version) { Die "Cargo.toml version $cargoVer != manifest.json version $($manifest.version)" }
$Version = $manifest.version; $report.version = $Version
if ($Sign -and -not (Test-Path -LiteralPath $SignScript)) { Die "-Sign given but SignScript not found: $SignScript" }
if ($Clean) {
  foreach ($d in $OutDir, $Stage) { if (Test-Path -LiteralPath $d) { Remove-Item -LiteralPath $d -Recurse -Force -Confirm:$false } }
}
New-Item -ItemType Directory -Force -Path $Build, $Stage, $OutDir | Out-Null

$want = @(if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'x64' })
if ($X64 -and $want -notcontains 'x64') { $want += 'x64' }
if ($Arm64 -and $want -notcontains 'arm64') { $want += 'arm64' }

# --- 2. build, check, (sign), stage, pack per arch --------------------------------------------
$env:CARGO_TARGET_DIR = $TargetDir
foreach ($arch in $want) {
  $triple = $Arches[$arch].triple
  Step "2. $arch ($triple)"
  $log = Join-Path $Build "cargo_$arch.log"
  Push-Location $Repo
  try { $rc = Invoke-Native 'cargo' @('build', '--release', '--locked', '--target', $triple) $log } finally { Pop-Location }
  $exe = Join-Path $TargetDir "$triple\release\programmer.exe"
  if ($rc -ne 0 -or -not (Test-Path -LiteralPath $exe)) { Die "cargo build failed for $triple (exit $rc); see $log" }
  $machine = Get-PeMachine $exe
  if ($machine -ne $Arches[$arch].machine) { Die ("{0} exe has PE machine 0x{1:X4}" -f $arch, $machine) }
  $hs = Invoke-Handshake @($exe, 'serve') @()
  if (-not $hs.ok) { Die "$arch handshake failed: $($hs.error)" }
  Assert-ToolsMatch (Join-Path $Here 'manifest.json') $hs "programmer $arch"
  if ($hs.serverInfo.version -ne $Version) { Die "serverInfo.version $($hs.serverInfo.version) != $Version" }
  $entry = [ordered]@{
    triple = $triple; exe_sha256 = (Get-Sha $exe); exe_size = (Get-Item -LiteralPath $exe).Length
    pe_machine = ('0x{0:X4}' -f $machine); server_info = $hs.serverInfo; protocol = $hs.protocolVersion
    tool_count = $hs.toolCount; annotated_tools = $hs.annotatedCount; titled_tools = $hs.titledCount
  }
  if ($hs.annotatedCount -lt $hs.toolCount) {
    Warn "$arch`: $($hs.toolCount - $hs.annotatedCount) of $($hs.toolCount) tools have no MCP annotations (readOnlyHint/destructiveHint)."
  }
  Write-Host ("{0}: {1} tools, serverInfo {2} {3}, PE {4}" -f $arch, $hs.toolCount, $hs.serverInfo.name, $hs.serverInfo.version, $entry.pe_machine)

  $dir = Join-Path $Stage $arch
  if (Test-Path -LiteralPath $dir) { Remove-Item -LiteralPath $dir -Recurse -Force -Confirm:$false }
  New-Item -ItemType Directory -Force -Path (Join-Path $dir 'server') | Out-Null
  foreach ($f in $BundleFiles) { Copy-Item -LiteralPath (Join-Path $Here $f) -Destination $dir }
  foreach ($f in $RepoFiles) { Copy-Item -LiteralPath (Join-Path $Repo $f) -Destination $dir }
  $stagedExe = Join-Path $dir 'server\programmer.exe'
  Copy-Item -LiteralPath $exe -Destination $stagedExe

  if ($Sign) {
    Write-Host "signing $stagedExe with $SignScript"
    & pwsh -NoProfile -File $SignScript -Path $stagedExe
    if ($LASTEXITCODE -ne 0) { Die "signing failed for $stagedExe" }
    $sig = Get-AuthenticodeSignature -LiteralPath $stagedExe
    if ($sig.Status -ne 'Valid') { Die "programmer.exe signature status is $($sig.Status) after signing" }
    $entry.signature = [ordered]@{ status = "$($sig.Status)"; signer = $sig.SignerCertificate.Subject }
    $entry.signed_exe_sha256 = Get-Sha $stagedExe
  } else {
    $entry.signature = 'unsigned'
  }

  $rc = Invoke-Native 'mcpb' @('validate', (Join-Path $dir 'manifest.json')) (Join-Path $Build "mcpb_validate_$arch.log")
  if ($rc -ne 0) { Die "mcpb validate failed; see build\mcpb_validate_$arch.log" }
  $name = "programmer-$Version-win-$arch.mcpb"
  $out = Join-Path $OutDir $name
  if (Test-Path -LiteralPath $out) { Remove-Item -LiteralPath $out -Force -Confirm:$false }
  $rc = Invoke-Native 'mcpb' @('pack', $dir, $out) (Join-Path $Build "mcpb_pack_$arch.log")
  if ($rc -ne 0 -or -not (Test-Path -LiteralPath $out)) { Die "mcpb pack failed; see build\mcpb_pack_$arch.log" }

  if ($Sign -and $McpbCert -and $McpbKey) {
    $sa = @('sign', $out, '--cert', $McpbCert, '--key', $McpbKey)
    if ($McpbIntermediate) { $sa += '--intermediate'; $sa += $McpbIntermediate }
    $rc = Invoke-Native 'mcpb' $sa (Join-Path $Build "mcpb_sign_$arch.log")
    if ($rc -ne 0) { Die "mcpb sign failed; see build\mcpb_sign_$arch.log" }
  } elseif ($Sign) {
    Warn "$name`: exe signed, but no -McpbCert/-McpbKey given, so the .mcpb itself is not signed."
  }

  # Archive-root check, then an install-like test: unpack and handshake the unpacked exe.
  $z = [IO.Compression.ZipFile]::OpenRead($out)
  try { $entries = @($z.Entries | ForEach-Object { $_.FullName -replace '\\', '/' }) } finally { $z.Dispose() }
  foreach ($need in @('manifest.json', 'icon.png', 'LICENSE', 'NOTICE', 'PRIVACY.md', $manifest.server.entry_point)) {
    if ($entries -notcontains $need) { Die "$name is missing archive-root entry '$need'" }
  }
  $tmp = Join-Path ([IO.Path]::GetTempPath()) ("programmer-unpack-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
  try {
    $rc = Invoke-Native 'mcpb' @('unpack', $out, $tmp) (Join-Path $Build "mcpb_unpack_$arch.log")
    if ($rc -ne 0) { Die "mcpb unpack failed; see build\mcpb_unpack_$arch.log" }
    $uhs = Invoke-Handshake @((Join-Path $tmp 'server\programmer.exe'), 'serve') @("PROGRAMMER_STATE_DIR=$(Join-Path $tmp 'state')")
    if (-not $uhs.ok) { Die "unpacked $name handshake failed: $($uhs.error)" }
    Assert-ToolsMatch (Join-Path $tmp 'manifest.json') $uhs "unpacked $name"
  } finally { Remove-Item -LiteralPath $tmp -Recurse -Force -Confirm:$false -ErrorAction SilentlyContinue }

  $info = (& mcpb info $out 2>&1) -join ' '
  $report.builds[$arch] = $entry
  $report.bundles[$name] = [ordered]@{
    size = (Get-Item -LiteralPath $out).Length; sha256 = (Get-Sha $out); entries = $entries.Count
    unpacked_handshake = "$($uhs.toolCount) tools, $($uhs.serverInfo.name) $($uhs.serverInfo.version)"
    mcpb_signed = ($info -notmatch 'Not signed')
  }
  Write-Host ("{0}  {1} entries  {2}" -f $name, $entries.Count, $report.bundles[$name].sha256)
}

# --- 3. checksums -----------------------------------------------------------------------------
Step '3. checksums'
$sums = foreach ($f in Get-ChildItem -LiteralPath $OutDir -Filter "programmer-$Version-*.mcpb" -File | Sort-Object Name) {
  "$(Get-Sha $f.FullName)  $($f.Name)"
}
[IO.File]::WriteAllText((Join-Path $OutDir 'SHA256SUMS.txt'), ((@($sums) -join "`n") + "`n"), $Utf8NoBom)
Save-Report
Write-Host "Done. $OutDir ; report: $(Join-Path $Build 'pack-report.json')" -ForegroundColor Green
if ($report.warnings.Count -gt 0) { Write-Host "$($report.warnings.Count) warning(s); see report." -ForegroundColor Yellow }
exit 0
