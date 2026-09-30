param(
    [Parameter(Mandatory = $true)]
    [string]$UefiDirectory,
    [Parameter(Mandatory = $true)]
    [string]$FirmwarePath,
    [int]$TimeoutSeconds = 30
)

$qemu = Get-Command qemu-system-x86_64.exe -ErrorAction SilentlyContinue
if ($null -eq $qemu) {
    $fallback = Join-Path $env:ProgramFiles "qemu\qemu-system-x86_64.exe"
    if (-not (Test-Path $fallback)) {
        throw "qemu-system-x86_64.exe was not found on PATH or under Program Files\qemu."
    }
    $qemuPath = $fallback
} else {
    $qemuPath = $qemu.Source
}
if (-not (Test-Path $FirmwarePath)) {
    throw "OVMF firmware image was not found: $FirmwarePath"
}

$logPath = Join-Path $env:TEMP "nebulaos-uefi-smoke.log"
Remove-Item $logPath -ErrorAction SilentlyContinue
$fatDirectory = (Resolve-Path $UefiDirectory).Path
$arguments = @(
    "-bios", (Resolve-Path $FirmwarePath).Path,
    "-drive", "format=raw,file=fat:rw:$fatDirectory",
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

if (-not (Test-Path $logPath) -or (Get-Content $logPath -Raw) -notmatch "NebulaOS x86_64 kernel booted") {
    $contents = if (Test-Path $logPath) { Get-Content $logPath -Raw } else { "<no serial output>" }
    throw "UEFI image did not reach kernel_main. Serial output: $contents"
}

Write-Output "UEFI smoke test passed."