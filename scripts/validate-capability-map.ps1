param(
    [string]$Source = (Join-Path $PSScriptRoot "..\src\tools\mod.rs"),
    [string]$Map = (Join-Path $PSScriptRoot "..\skills\programmer\references\capability-map.md"),
    [string]$Readme = (Join-Path $PSScriptRoot "..\README.md")
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$sourceText = Get-Content -Raw -LiteralPath $Source
$sourceNames = @(
    [regex]::Matches($sourceText, '"name"\s*:\s*"([a-zA-Z0-9_-]+)"') |
        ForEach-Object { $_.Groups[1].Value }
)
$sourceUnique = @($sourceNames | Sort-Object -Unique)

if ($sourceNames.Count -ne $sourceUnique.Count) {
    throw "The source registry contains duplicate tool names."
}

$mapText = Get-Content -Raw -LiteralPath $Map
$match = [regex]::Match($mapText, '(?s)<!-- TOOL_MAP_START -->(.*?)<!-- TOOL_MAP_END -->')
if (-not $match.Success) {
    throw "The capability map markers are missing."
}

$mapNames = @(
    [regex]::Matches($match.Groups[1].Value, '`([a-zA-Z0-9_-]+)`') |
        ForEach-Object { $_.Groups[1].Value }
)
$mapUnique = @($mapNames | Sort-Object -Unique)

if ($mapNames.Count -ne $mapUnique.Count) {
    throw "The capability map contains duplicate tool names."
}

$sections = @([regex]::Matches(
    $match.Groups[1].Value,
    '(?ms)^## (.+?) \((\d+)\)\s*$\s*(.*?)(?=^## |\z)'
))
if (-not $sections.Count) {
    throw "The capability map has no counted ability groups."
}

$declaredCounts = @()
foreach ($section in $sections) {
    $name = $section.Groups[1].Value
    $declared = [int]$section.Groups[2].Value
    $actual = [regex]::Matches($section.Groups[3].Value, '`([a-zA-Z0-9_-]+)`').Count
    if ($actual -ne $declared) {
        throw "Ability group '$name' declares $declared tools but contains $actual."
    }
    $declaredCounts += $declared
}

$readmeText = Get-Content -Raw -LiteralPath $Readme
$readmeCounts = @(
    [regex]::Matches($readmeText, '(?m)^\| \*\*[^|]+\*\* \| (\d+) \|') |
        ForEach-Object { [int]$_.Groups[1].Value }
)
if (($readmeCounts -join ',') -ne ($declaredCounts -join ',')) {
    throw "README ability counts do not match the capability map."
}

if (($declaredCounts | Measure-Object -Sum).Sum -ne $mapUnique.Count) {
    throw "Ability group totals do not match the unique mapped tool count."
}

$missing = @(Compare-Object -ReferenceObject $mapUnique -DifferenceObject $sourceUnique |
    Where-Object SideIndicator -eq '=>' | ForEach-Object InputObject)
$extra = @(Compare-Object -ReferenceObject $mapUnique -DifferenceObject $sourceUnique |
    Where-Object SideIndicator -eq '<=' | ForEach-Object InputObject)

if ($missing.Count -or $extra.Count) {
    throw "Capability map drift. Missing from map: $($missing -join ', '). Extra in map: $($extra -join ', ')."
}

Write-Output "Capability map matches all $($sourceUnique.Count) unique source tool definitions."
