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
call :create_iso x86_64
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
    set "TARGET=i686-unknown-none"
    set "KERNEL_ELF=%BUILD_DIR%\nebulaos_x86.elf"
    set "KERNEL_BIN=%BUILD_DIR%\nebulaos_x86.bin"
) else (
    set "TARGET=x86_64-unknown-none"
    set "KERNEL_ELF=%BUILD_DIR%\nebulaos_x86_64.elf"
    set "KERNEL_BIN=%BUILD_DIR%\nebulaos_x86_64.bin"
)

echo [INFO] Building NebulaOS for %arch%...
echo [INFO]   Building Rust kernel for %TARGET%...
cargo build --target "%TARGET%" --release
if errorlevel 1 (
    echo [ERROR] Rust build failed for %arch%
    exit /b 1
)

if exist "target\%TARGET%\release\kernel" (
    copy /Y "target\%TARGET%\release\kernel" "%KERNEL_ELF%" >nul
    if errorlevel 1 (
        echo [ERROR] Failed to copy the kernel ELF
        exit /b 1
    )
) else if exist "target\%TARGET%\release\libkernel.a" (
    echo [ERROR] Only static lib produced, need full kernel binary
    exit /b 1
)

if not exist "%KERNEL_ELF%" (
    echo [ERROR] Kernel ELF not found after build
    exit /b 1
)

echo [INFO]   Converting to binary...
if /i "%arch%"=="x86" (
    set "OBJCOPY=i686-linux-gnu-objcopy"
) else (
    set "OBJCOPY=x86_64-linux-gnu-objcopy"
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
    rem Build NebulaBoot for x86 (BIOS)
    echo [INFO] Building NebulaBoot for x86...
    nasm -f bin "boot\nebula_boot\x86\boot.asm" -o "%BUILD_DIR%\nebula_boot_x86.bin"
    if errorlevel 1 (
        echo [ERROR] NebulaBoot x86 build failed
        exit /b 1
    )
    
    rem Copy bootloader to ISO
    copy /Y "%BUILD_DIR%\nebula_boot_x86.bin" "%ISO_DIR%\boot\nebulaos\nebula_boot_x86.bin" >nul
    if errorlevel 1 (
        echo [ERROR] Failed to copy NebulaBoot x86
        exit /b 1
    )
    
    rem Create BIOS-bootable ISO with El Torito
    xorriso -as mkisofs -R -b boot/nebulaos/nebula_boot_x86.bin -no-emul-boot -boot-load-size 4 -boot-info-table -o "%ISO_FILE%" "%ISO_DIR%"
    if errorlevel 1 (
        echo [ERROR] ISO creation failed for x86
        exit /b 1
    )
) else (
    rem For x86_64, create UEFI-bootable ISO
    echo [INFO] Building NebulaBoot for x86_64 (UEFI)...
    
    rem We need to build the UEFI bootloader
    rem For now, create a minimal UEFI bootloader that chains to kernel
    if not exist "%ISO_DIR%\EFI\BOOT" mkdir "%ISO_DIR%\EFI\BOOT"
    if not exist "%ISO_DIR%\EFI\NEBULA" mkdir "%ISO_DIR%\EFI\NEBULA"
    
    nasm -f win64 "boot\nebula_boot\x86_64\boot.asm" -o "%BUILD_DIR%\boot_x86_64.obj"
    if errorlevel 1 (
        echo [ERROR] NebulaBoot x86_64 build failed
        exit /b 1
    )
    
    rem Link UEFI executable
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
    
    rem Also copy kernel to EFI/NEBULA for the bootloader to find
    copy /Y "%KERNEL_FILE%" "%ISO_DIR%\EFI\NEBULA\nebulaos_x86_64.elf" >nul
    if errorlevel 1 (
        echo [ERROR] Failed to copy kernel to EFI
        exit /b 1
    )
    
    rem Create UEFI-bootable ISO
    xorriso -as mkisofs -R -eltorito-alt-boot -e EFI/BOOT/BOOTX64.EFI -no-emul-boot -o "%ISO_FILE%" "%ISO_DIR%"
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
    set "ISO_FILE=%ISO_DIR%\nebulaos_x86.iso"
) else (
    set "QEMU_BIN=qemu-system-x86_64"
    set "ISO_FILE=%ISO_DIR%\nebulaos_x86_64.iso"
)

if not exist "%ISO_FILE%" (
    echo [ERROR] ISO not found: %ISO_FILE%
    echo [ERROR] Build the ISO first: build.bat iso
    exit /b 1
)

echo [INFO] Running NebulaOS (%arch%) in QEMU...
echo [INFO]   ISO: %ISO_FILE%
echo [INFO]   Press Ctrl+Alt+G to release mouse, Ctrl+Alt+Q to quit
"%QEMU_BIN%" -cdrom "%ISO_FILE%" -m 256M -serial stdio
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
