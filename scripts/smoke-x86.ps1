param(
    [Parameter(Mandatory = $true)]
    [string]$IsoPath,
    [int]$TimeoutSeconds = 20
)

$qemu = Get-Command qemu-system-i386.exe -ErrorAction SilentlyContinue
if ($null -eq $qemu) {
    $fallback = Join-Path $env:ProgramFiles "qemu\qemu-system-i386.exe"
    if (-not (Test-Path $fallback)) {
        throw "qemu-system-i386.exe was not found on PATH or under Program Files\qemu."
    }
    $qemuPath = $fallback
} else {
    $qemuPath = $qemu.Source
}

$logPath = Join-Path $env:TEMP "nebulaos-x86-smoke.log"
Remove-Item $logPath -ErrorAction SilentlyContinue
$arguments = @(
    "-cdrom", (Resolve-Path $IsoPath).Path,
    "-m", "256M",
    "-display", "none",
    "-monitor", "none",
    "-serial", "file:$logPath",
    "-no-reboot",
    "-no-shutdown"
)
$process = Start-Process -FilePath $qemuPath -ArgumentList $arguments -PassThru -WindowStyle Hidden
if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
    $process.Kill()
    $process.WaitForExit()
}

if (-not (Test-Path $logPath) -or (Get-Content $logPath -Raw) -notmatch "NebulaOS x86 kernel booted") {
    $contents = if (Test-Path $logPath) { Get-Content $logPath -Raw } else { "<no serial output>" }
    throw "BIOS ISO did not reach kernel_main. Serial output: $contents"
}

Write-Output "BIOS ISO smoke test passed."