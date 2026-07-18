<#
  render-hooks.ps1 - Render the opt-in hook fragments with this plugin's
  absolute install path, writing host-ready JSON into rendered-hooks/.

  The fragments in hooks/opt-in/ are inert templates: they contain a
  __PLUGIN_ROOT__ token and are never auto-loaded. Rendering is the explicit
  user step that turns a reviewed template into wire-able hook JSON.

  Usage:  pwsh -File scripts/render-hooks.ps1   (from the plugin root, or pass -PluginRoot)
#>
[CmdletBinding()]
param(
  [string]$PluginRoot = (Split-Path -Parent $PSScriptRoot)
)
$ErrorActionPreference = 'Stop'

$root = (Resolve-Path $PluginRoot).Path -replace '\\', '/'
$outDir = Join-Path $PluginRoot 'rendered-hooks'
New-Item -ItemType Directory -Force $outDir | Out-Null

foreach ($fragment in Get-ChildItem (Join-Path $PluginRoot 'hooks/opt-in') -Filter '*-hooks.fragment.json') {
  $rendered = (Get-Content $fragment.FullName -Raw) -replace '__PLUGIN_ROOT__', $root
  $target = Join-Path $outDir ($fragment.Name -replace '\.fragment', '')
  Set-Content -Path $target -Value $rendered -Encoding utf8
  Write-Host "rendered $($fragment.Name) -> $target"
}
Write-Host "Review the rendered JSON, then merge the entries you want into your host's hook settings (for Claude Code: ~/.claude/settings.json)."
