param(
    [string]$Base,
    [string]$Head,
    [string]$PullRequest = "false"
)

$ErrorActionPreference = "Stop"
$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or -not $repoRoot) {
    throw "Could not determine the repository root."
}

Set-Location -LiteralPath $repoRoot
if ($Base -and $Head) {
    $separator = if ($PullRequest -eq "true") { "..." } else { ".." }
    $range = "$Base$separator$Head"
    $changed = @(& git diff --name-only --diff-filter=ACMR $range -- "*.md")
    if ($LASTEXITCODE -ne 0) {
        throw "Could not list changed Markdown files for $range."
    }
} else {
    $changed = @(& git diff --name-only --diff-filter=ACMR HEAD -- "*.md")
    if ($LASTEXITCODE -ne 0) {
        throw "Could not list changed tracked Markdown files."
    }
    $untracked = @(& git ls-files --others --exclude-standard -- "*.md")
    if ($LASTEXITCODE -ne 0) {
        throw "Could not list untracked Markdown files."
    }
    $changed += $untracked
}

$files = @($changed | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) })
$issues = [System.Collections.Generic.List[string]]::new()
$linkPattern = [regex]'\]\((?<target><[^>]+>|[^)\s]+)(?:\s+[^)]*)?\)'
$fencePattern = '^\s*([~]{3,}|[' + [char]96 + ']{3,})'

foreach ($relativePath in $files) {
    $fullPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $relativePath))
    $lines = Get-Content -LiteralPath $fullPath
    $fence = $null

    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = $lines[$index]
        $lineNumber = $index + 1

        if ($line -match $fencePattern) {
            $marker = $Matches[1]
            if ($null -eq $fence) {
                $fence = $marker
                continue
            }
            if ($marker[0] -eq $fence[0] -and $marker.Length -ge $fence.Length) {
                $fence = $null
                continue
            }
        }
        if ($null -ne $fence) {
            continue
        }

        foreach ($match in $linkPattern.Matches($line)) {
            $target = $match.Groups["target"].Value
            if ($target.StartsWith("<") -and $target.EndsWith(">")) {
                $target = $target.Substring(1, $target.Length - 2)
            }
            if (-not $target -or $target.StartsWith("#") -or $target -match '^(?:[a-z][a-z0-9+.-]*:|//)') {
                continue
            }

            $pathPart = ($target -split "[#?]", 2)[0]
            if (-not $pathPart) {
                continue
            }
            $decodedPath = [System.Uri]::UnescapeDataString($pathPart)
            $sourceDirectory = Split-Path -Parent $fullPath
            $destination = [System.IO.Path]::GetFullPath((Join-Path $sourceDirectory $decodedPath))
            if (-not (Test-Path -LiteralPath $destination)) {
                [void]$issues.Add("${relativePath}:${lineNumber}: missing local link target '$target'")
            }
        }
    }

    if ($null -ne $fence) {
        [void]$issues.Add("${relativePath}: unclosed $($fence.Length)-character code fence")
    }
}

if ($issues.Count -gt 0) {
    $issues | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Output "Checked $($files.Count) changed Markdown file(s): local links and code fences are valid."
