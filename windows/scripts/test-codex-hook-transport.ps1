param(
    [string]$HookExe = (Join-Path $PSScriptRoot '..\target\release\anti-scrolling-notch-hook.exe')
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $HookExe -PathType Leaf)) {
    throw "Build the hook first; executable not found: $HookExe"
}

$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$pipeName = "anti-scrolling-notch-codex-$sid"
$utf8 = [System.Text.UTF8Encoding]::new($false)
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$projectionPath = Join-Path $repoRoot 'tests\compat\codex-hooks\captured-projections.jsonl'
if (-not (Test-Path -LiteralPath $projectionPath -PathType Leaf)) {
    throw "Sanitized loopback capture projection not found: $projectionPath"
}

function Start-HookProcess([string]$Payload) {
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = (Resolve-Path -LiteralPath $HookExe).Path
    $start.Arguments = '--codex-observer'
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    if (-not $process.Start()) { throw 'Hook process failed to start.' }
    $process.StandardInput.WriteLine($Payload)
    $process.StandardInput.Close()
    return $process
}

function Assert-NeutralExit($Process) {
    if (-not $Process.WaitForExit(5000)) { throw 'Hook process exceeded the 5-second test bound.' }
    $stdout = $Process.StandardOutput.ReadToEnd()
    $stderr = $Process.StandardError.ReadToEnd()
    if ($Process.ExitCode -ne 0) { throw "Hook exited with code $($Process.ExitCode)." }
    if ($stdout.Length -ne 0) { throw 'Observer wrote to stdout; Codex hook output must remain neutral.' }
    if ($stderr.Length -ne 0) { throw 'Observer wrote unexpected stderr.' }
}

$cases = @(Get-Content -LiteralPath $projectionPath | Where-Object { $_.Trim() } | ForEach-Object {
    $projection = $_ | ConvertFrom-Json
    if (-not $projection.hook_event_name) { throw 'Capture projection has no event name.' }
    @{ Name = $projection.hook_event_name; Payload = $_ }
})
if ($cases.Count -ne 4) {
    throw "Expected the four loopback-captured event projections; found $($cases.Count)."
}

foreach ($case in $cases) {
    $server = [System.IO.Pipes.NamedPipeServerStream]::new(
        $pipeName,
        [System.IO.Pipes.PipeDirection]::InOut,
        1,
        [System.IO.Pipes.PipeTransmissionMode]::Byte,
        [System.IO.Pipes.PipeOptions]::Asynchronous
    )
    try {
        $connect = $server.WaitForConnectionAsync()
        $process = Start-HookProcess $case.Payload
        if (-not $connect.Wait(5000)) { throw "No transport connection for $($case.Name)." }
        $reader = [System.IO.StreamReader]::new($server, $utf8, $false, 1024, $true)
        $line = $reader.ReadLine()
        Assert-NeutralExit $process
        $message = $line | ConvertFrom-Json
        if ($message.wire_version -ne 1 -or $message.event -ne $case.Name) {
            throw "Unexpected internal message for $($case.Name): $line"
        }
        $allowed = @('wire_version', 'event')
        $actual = @($message.PSObject.Properties.Name)
        $actualNames = ($actual | Sort-Object) -join ','
        $allowedNames = ($allowed | Sort-Object) -join ','
        if ($actualNames -cne $allowedNames) {
            throw "Unexpected data crossed the Codex adapter boundary: $line"
        }
        $process.Dispose()
        $reader.Dispose()
    }
    finally {
        $server.Dispose()
    }
    Write-Output "PASS $($case.Name): isolated pipe delivery, allowlisted fields, neutral exit"
}

# Events configured in the probe but never observed, and malformed stdin,
# must fail neutral before opening the backend pipe.
$negativeCases = @(
    @{ Name = 'unverified PreToolUse'; Payload = '{"hook_event_name":"PreToolUse"}' },
    @{ Name = 'unverified PostToolUse'; Payload = '{"hook_event_name":"PostToolUse"}' },
    @{ Name = 'unverified PreCompact'; Payload = '{"hook_event_name":"PreCompact"}' },
    @{ Name = 'unverified PostCompact'; Payload = '{"hook_event_name":"PostCompact"}' },
    @{ Name = 'unverified SubagentStart'; Payload = '{"hook_event_name":"SubagentStart"}' },
    @{ Name = 'unverified SubagentStop'; Payload = '{"hook_event_name":"SubagentStop"}' },
    @{ Name = 'unverified Interrupt'; Payload = '{"hook_event_name":"Interrupt"}' },
    @{ Name = 'unverified PermissionRequest'; Payload = '{"hook_event_name":"PermissionRequest"}' },
    @{ Name = 'malformed JSON'; Payload = 'not-json' }
)
foreach ($case in $negativeCases) {
    $server = [System.IO.Pipes.NamedPipeServerStream]::new(
        $pipeName,
        [System.IO.Pipes.PipeDirection]::InOut,
        1,
        [System.IO.Pipes.PipeTransmissionMode]::Byte,
        [System.IO.Pipes.PipeOptions]::Asynchronous
    )
    try {
        $connect = $server.WaitForConnectionAsync()
        $process = Start-HookProcess $case.Payload
        Assert-NeutralExit $process
        if ($connect.Wait(350)) { throw "$($case.Name) unexpectedly reached the backend pipe." }
        $process.Dispose()
    }
    finally {
        $server.Dispose()
    }
    Write-Output "PASS $($case.Name): neutral success without transport"
}

$existingPipe = Get-ChildItem -LiteralPath '\\.\pipe\' -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -eq $pipeName } |
    Select-Object -First 1
if ($existingPipe) {
    Write-Output 'SKIP absent-app bound: the app currently owns the Codex pipe.'
}
else {
    foreach ($case in $cases) {
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        $process = Start-HookProcess $case.Payload
        Assert-NeutralExit $process
        $clock.Stop()
        if ($clock.ElapsedMilliseconds -gt 1800) {
            throw "Absent-app relay for $($case.Name) exceeded its 1800 ms bound: $($clock.ElapsedMilliseconds) ms."
        }
        $process.Dispose()
        Write-Output "PASS absent-app $($case.Name): neutral success in $($clock.ElapsedMilliseconds) ms"
    }
}
