param(
    [string]$HookExe = (Join-Path $PSScriptRoot '..\target\release\coucou-hook.exe')
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $HookExe -PathType Leaf)) {
    throw "Build the hook first; executable not found: $HookExe"
}

$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$pipeName = "coucou-codex-$sid"
$utf8 = [System.Text.UTF8Encoding]::new($false)

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

$cases = @(
    @{ Name = 'SessionStart'; Payload = '{"hook_event_name":"SessionStart","source":"startup"}' },
    @{ Name = 'UserPromptSubmit'; Payload = '{"hook_event_name":"UserPromptSubmit"}' },
    @{ Name = 'Stop'; Payload = '{"hook_event_name":"Stop"}' },
    @{ Name = 'SessionEnd'; Payload = '{"hook_event_name":"SessionEnd","reason":"other"}' }
)

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

# An event that was configured in the probe but never observed, and malformed
# stdin, must both fail neutral before opening the backend pipe.
$negativeCases = @(
    @{ Name = 'unverified PreToolUse'; Payload = '{"hook_event_name":"PreToolUse"}' },
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
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    $process = Start-HookProcess '{"hook_event_name":"SessionStart"}'
    Assert-NeutralExit $process
    $clock.Stop()
    if ($clock.ElapsedMilliseconds -gt 1800) {
        throw "Absent-app relay exceeded its 1800 ms bound: $($clock.ElapsedMilliseconds) ms."
    }
    $process.Dispose()
    Write-Output "PASS absent-app bound: neutral success in $($clock.ElapsedMilliseconds) ms"
}
