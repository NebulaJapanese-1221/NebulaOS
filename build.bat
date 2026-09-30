@echo off
setlocal EnableExtensions DisableDelayedExpansion

for %%I in ("%~dp0.") do set "PROJECT_DIR=%%~fI"
set "BUILD_DIR=%PROJECT_DIR%\build"
set "ISO_DIR=%BUILD_DIR%\iso"
if not defined ARCH set "ARCH=all"
set "COMMAND=%~1"
if not defined COMMAND set "COMMAND=all"

pushd "%PROJECT_DIR%" || exit /b 1

if /i "%COMMAND%"=="clean" goto clean
if /i "%COMMAND%"=="x86" goto build_x86
if /i "%COMMAND%"=="x86_64" goto build_x86_64
if /i "%COMMAND%"=="iso" goto build_iso
if /i "%COMMAND%"=="run" goto run
goto build_all

:clean
echo [INFO] Cleaning build directory...
if exist "%BUILD_DIR%" (
    rmdir /s /q "%BUILD_DIR%"
    if errorlevel 1 goto failed
)
echo [INFO] Clean complete.
goto done

:require_cmd
where %~1 >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Required tool "%~1" is not installed or not on PATH.
    exit /b 1
)
exit /b 0

:ensure_nightly
rustup toolchain list 2>nul | findstr /R /C:"^nightly" >nul
if errorlevel 1 (
    echo [ERROR] Rust nightly is required. Install it with: rustup toolchain install nightly
    exit /b 1
)
exit /b 0

:build_x86
call :create_dirs
if errorlevel 1 goto failed
call :build_arch x86
if errorlevel 1 goto failed
goto done

:build_x86_64
call :create_dirs
if errorlevel 1 goto failed
call :build_arch x86_64
if errorlevel 1 goto failed
goto done

:build_iso
call :create_iso x86
if errorlevel 1 goto failed
if not exist "%BUILD_DIR%\nebulaos_x86_64.elf" echo [WARN] Skipping x86_64 ISO because its kernel has not been built.
if exist "%BUILD_DIR%\nebulaos_x86_64.elf" call :create_iso x86_64
if errorlevel 1 goto failed
goto done

:run
call :run_qemu x86
goto done

:build_all
call :create_dirs
if errorlevel 1 goto failed
call :build_arch x86
if errorlevel 1 goto failed
call :build_arch x86_64
if errorlevel 1 goto failed
goto done

:create_dirs
if not exist "%BUILD_DIR%" mkdir "%BUILD_DIR%"
if errorlevel 1 exit /b 1
if not exist "%ISO_DIR%\boot\nebulaos" mkdir "%ISO_DIR%\boot\nebulaos"
if not exist "%ISO_DIR%\EFI\BOOT" mkdir "%ISO_DIR%\EFI\BOOT"
if not exist "%ISO_DIR%\EFI\NEBULA" mkdir "%ISO_DIR%\EFI\NEBULA"
exit /b %ERRORLEVEL%

:build_arch
set "arch=%~1"
if /i "%arch%"=="x86" (
    set "TARGET=%PROJECT_DIR%\targets\x86.json"
    set "TARGET_TRIPLE=x86"
    set "KERNEL_ELF=%BUILD_DIR%\nebulaos_x86.elf"
    set "KERNEL_BIN=%BUILD_DIR%\nebulaos_x86.bin"
) else (
    set "TARGET=%PROJECT_DIR%\targets\x86_64.json"
    set "TARGET_TRIPLE=x86_64"
    set "KERNEL_ELF=%BUILD_DIR%\nebulaos_x86_64.elf"
    set "KERNEL_BIN=%BUILD_DIR%\nebulaos_x86_64.bin"
)

call :require_cmd cargo
if errorlevel 1 goto failed
call :ensure_nightly
if errorlevel 1 goto failed
call :require_cmd nasm
if errorlevel 1 goto failed

echo [INFO] Building NebulaOS for %arch%...
echo [INFO]   Building Rust kernel for %TARGET%...
cargo +nightly -Zjson-target-spec -Zbuild-std=core build --manifest-path "%PROJECT_DIR%\kernel\Cargo.toml" --target "%TARGET%" --release
if errorlevel 1 (
    echo [ERROR] Rust build failed for %arch%
    exit /b 1
)

set "FOUND_KERNEL="
for /r "%PROJECT_DIR%\target\%TARGET_TRIPLE%\release" %%F in (kernel.exe kernel) do (
    if exist "%%~fF" (
        copy /Y "%%~fF" "%KERNEL_ELF%" >nul
        if errorlevel 1 (
            echo [ERROR] Failed to copy the kernel ELF
            exit /b 1
        )
        set "FOUND_KERNEL=1"
        goto kernel_found
    )
)

if not defined FOUND_KERNEL (
    echo [ERROR] Kernel ELF not found after build
    exit /b 1
)

:kernel_found
if not exist "%KERNEL_ELF%" (
    echo [ERROR] Kernel ELF not found after build
    exit /b 1
)

echo [INFO]   Converting to binary...
if /i "%arch%"=="x86" (
    where i686-linux-gnu-objcopy >nul 2>nul
    if not errorlevel 1 (
        set "OBJCOPY=i686-linux-gnu-objcopy"
    ) else (
        set "OBJCOPY=objcopy"
    )
) else (
    where x86_64-linux-gnu-objcopy >nul 2>nul
    if not errorlevel 1 (
        set "OBJCOPY=x86_64-linux-gnu-objcopy"
    ) else (
        set "OBJCOPY=objcopy"
    )
)
"%OBJCOPY%" -O binary "%KERNEL_ELF%" "%KERNEL_BIN%"
if errorlevel 1 (
    echo [ERROR] Failed to convert the kernel to binary
    exit /b 1
)
echo [INFO]   Build complete: %KERNEL_BIN%
exit /b 0

:create_iso
set "arch=%~1"
set "KERNEL_FILE=%BUILD_DIR%\nebulaos_%arch%.elf"
set "ISO_FILE=%ISO_DIR%\nebulaos_%arch%.iso"

if not exist "%KERNEL_FILE%" (
    echo [ERROR] Kernel ELF not found: %KERNEL_FILE%
    echo [ERROR] Build the kernel first: build.bat %arch%
    exit /b 1
)

if not exist "%ISO_DIR%\boot\nebulaos" mkdir "%ISO_DIR%\boot\nebulaos"
if errorlevel 1 exit /b 1
copy /Y "%KERNEL_FILE%" "%ISO_DIR%\boot\nebulaos\nebulaos_%arch%.elf" >nul
if errorlevel 1 (
    echo [ERROR] Failed to copy the kernel into the ISO
    exit /b 1
)

if /i "%arch%"=="x86" (
    echo [INFO] Building NebulaBoot for x86...
    call :require_cmd nasm
    if errorlevel 1 goto failed
    call :require_cmd xorriso
    if errorlevel 1 goto failed
    nasm -f bin "boot\nebula_boot\x86\boot.asm" -o "%BUILD_DIR%\nebula_boot_x86.bin"
    if errorlevel 1 (
        echo [ERROR] NebulaBoot x86 build failed
        exit /b 1
    )
    copy /B /Y "%BUILD_DIR%\nebula_boot_x86.bin"+"%BUILD_DIR%\nebulaos_x86.bin" "%BUILD_DIR%\boot_image_x86.bin" >nul
    if errorlevel 1 (
        echo [ERROR] Failed to append the x86 kernel to NebulaBoot
        exit /b 1
    )
    copy /Y "%BUILD_DIR%\boot_image_x86.bin" "%ISO_DIR%\boot\nebulaos\nebula_boot_x86.bin" >nul
    if errorlevel 1 (
        echo [ERROR] Failed to copy NebulaBoot x86
        exit /b 1
    )
    xorriso -as mkisofs -R -b boot/nebulaos/nebula_boot_x86.bin -no-emul-boot -boot-load-size 4 -boot-info-table -o "%ISO_FILE%" "build/iso"
    if errorlevel 1 (
        echo [ERROR] ISO creation failed for x86
        exit /b 1
    )
) else (
    echo [INFO] Building NebulaBoot for x86_64 UEFI...
    call :require_cmd nasm
    if errorlevel 1 goto failed
    call :require_cmd x86_64-w64-mingw32-gcc
    if errorlevel 1 goto failed
    call :require_cmd xorriso
    if errorlevel 1 goto failed
    nasm -f win64 "boot\nebula_boot\x86_64\boot.asm" -o "%BUILD_DIR%\boot_x86_64.obj"
    if errorlevel 1 (
        echo [ERROR] NebulaBoot x86_64 build failed
        exit /b 1
    )
    x86_64-w64-mingw32-gcc -nostdlib -Wl,-entry,efi_main -Wl,-subsystem,efi_application -o "%BUILD_DIR%\bootx64.efi" "%BUILD_DIR%\boot_x86_64.obj"
    if errorlevel 1 (
        echo [ERROR] NebulaBoot x86_64 link failed
        exit /b 1
    )
    copy /Y "%BUILD_DIR%\bootx64.efi" "%ISO_DIR%\EFI\BOOT\BOOTX64.EFI" >nul
    if errorlevel 1 (
        echo [ERROR] Failed to copy UEFI bootloader
        exit /b 1
    )
    copy /Y "%KERNEL_FILE%" "%ISO_DIR%\EFI\NEBULA\nebulaos_x86_64.elf" >nul
    if errorlevel 1 (
        echo [ERROR] Failed to copy kernel to EFI
        exit /b 1
    )
    xorriso -as mkisofs -R -eltorito-alt-boot -e EFI/BOOT/BOOTX64.EFI -no-emul-boot -o "%ISO_FILE%" "build/iso"
    if errorlevel 1 (
        echo [ERROR] ISO creation failed for x86_64
        exit /b 1
    )
)

echo [INFO] ISO created: %ISO_FILE%
exit /b 0

:run_qemu
set "arch=%~1"
if /i "%arch%"=="x86" (
    set "QEMU_BIN=qemu-system-i386"
    set "BOOT_FILE=%BUILD_DIR%\nebulaos_x86.elf"
    set "BOOT_OPTION=-kernel"
) else (
    set "QEMU_BIN=qemu-system-x86_64"
    set "BOOT_FILE=%ISO_DIR%\nebulaos_x86_64.iso"
    set "BOOT_OPTION=-cdrom"
)

if not exist "%BOOT_FILE%" (
    echo [ERROR] Boot image not found: %BOOT_FILE%
    echo [ERROR] Build the kernel first: build.bat x86
    exit /b 1
)

call :require_cmd "%QEMU_BIN%"
if errorlevel 1 goto failed

echo [INFO] Running NebulaOS (%arch%) in QEMU...
echo [INFO]   Image: %BOOT_FILE%
echo [INFO]   Press Ctrl+Alt+G to release mouse, Ctrl+Alt+Q to quit
"%QEMU_BIN%" %BOOT_OPTION% "%BOOT_FILE%" -m 256M -serial stdio
exit /b %ERRORLEVEL%

:failed
echo [ERROR] Build failed.
popd
endlocal
exit /b 1

:done
popd
endlocal
exit /b 0
