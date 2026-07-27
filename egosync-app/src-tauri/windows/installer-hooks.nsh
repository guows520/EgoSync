; EgoSync NSIS Installer Hooks
;
; 清理在文件复制前执行，并且 fail-closed：无法确认进程退出或目标文件可写时中止安装/卸载。

!macro NSIS_HOOK_PREINSTALL
  Push $0
  Push $1

  InitPluginsDir
  ClearErrors
  FileOpen $0 "$PLUGINSDIR\egosync-cleanup.ps1" w
  IfErrors egosync_preinstall_script_error
  FileWrite $0 "param([string]$$InstallDir)$\r$\n"
  FileWrite $0 "$$ErrorActionPreference = 'Stop'$\r$\n"
  FileWrite $0 "$$LogFile = Join-Path $$env:TEMP 'egosync-install-cleanup.log'$\r$\n"
  FileWrite $0 "function Log([string]$$Msg) { Add-Content -Path $$LogFile -Value $$Msg -ErrorAction SilentlyContinue }$\r$\n"
  FileWrite $0 "try {$\r$\n"
  FileWrite $0 "  Log ('=== EgoSync pre-install cleanup ' + (Get-Date -Format o) + ' ===')$\r$\n"
  FileWrite $0 "  if (-not (Test-Path -LiteralPath $$InstallDir)) { Log 'Fresh install, no cleanup needed'; exit 0 }$\r$\n"
  FileWrite $0 "  $$InstallDirNorm = (Resolve-Path -LiteralPath $$InstallDir -ErrorAction Stop).Path.TrimEnd('\')$\r$\n"
  FileWrite $0 "  $$targetFiles = @((Join-Path $$InstallDirNorm 'EgoSync.exe'), (Join-Path $$InstallDirNorm 'resources\opencode.exe'))$\r$\n"
  FileWrite $0 "  $$targetNorm = $$targetFiles | ForEach-Object { [IO.Path]::GetFullPath($$_).ToLowerInvariant() }$\r$\n"
  FileWrite $0 "  $$procs = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $$_.ExecutablePath -and ($$targetNorm -contains ([IO.Path]::GetFullPath($$_.ExecutablePath).ToLowerInvariant())) })$\r$\n"
  FileWrite $0 "  foreach ($$p in $$procs) { Log ('Found PID ' + $$p.ProcessId + ' path=' + $$p.ExecutablePath); Stop-Process -Id $$p.ProcessId -Force -ErrorAction SilentlyContinue }$\r$\n"
  FileWrite $0 "  $$deadline = (Get-Date).AddSeconds(15)$\r$\n"
  FileWrite $0 "  do { Start-Sleep -Milliseconds 250; $$alive = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $$_.ExecutablePath -and ($$procs.ProcessId -contains $$_.ProcessId) -and $$targetNorm.Contains(([IO.Path]::GetFullPath($$_.ExecutablePath).ToLowerInvariant())) }) } while ($$alive.Count -gt 0 -and (Get-Date) -lt $$deadline)$\r$\n"
  FileWrite $0 "  if ($$alive.Count -gt 0) { foreach ($$p in $$alive) { Start-Process -FilePath 'taskkill.exe' -ArgumentList @('/F', '/T', '/PID', [string]$$p.ProcessId) -Wait -WindowStyle Hidden; Log ('taskkill requested PID ' + $$p.ProcessId) } }$\r$\n"
  FileWrite $0 "  Start-Sleep -Seconds 1$\r$\n"
  FileWrite $0 "  $$stillAlive = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $$_.ExecutablePath -and ($$procs.ProcessId -contains $$_.ProcessId) -and $$targetNorm.Contains(([IO.Path]::GetFullPath($$_.ExecutablePath).ToLowerInvariant())) })$\r$\n"
  FileWrite $0 "  if ($$stillAlive.Count -gt 0) { throw ('Processes still alive: ' + ($$stillAlive.ProcessId -join ', ')) }$\r$\n"
  FileWrite $0 "  foreach ($$exe in $$targetFiles) { if (Test-Path -LiteralPath $$exe) { $$fs = [IO.File]::Open($$exe, 'Open', 'Write', 'None'); $$fs.Dispose() } }$\r$\n"
  FileWrite $0 "  Log 'SUCCESS: cleanup completed, files writable'; exit 0$\r$\n"
  FileWrite $0 "} catch { Log ('FAILED: ' + $$_); exit 1 }$\r$\n"
  FileClose $0
  Goto egosync_preinstall_execute

egosync_preinstall_file_error:
  MessageBox MB_OK|MB_ICONSTOP "旧版 EgoSync 或 sidecar 仍占用安装文件。请关闭 EgoSync 后重试。详细日志：%TEMP%\egosync-install-cleanup.log"
  Abort

egosync_preinstall_script_error:
  MessageBox MB_OK|MB_ICONSTOP "无法创建安装前清理脚本。请检查临时目录权限后重试。"
  Abort

egosync_preinstall_execute:
  nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\egosync-cleanup.ps1" "$INSTDIR"'
  Pop $0
  Delete "$PLUGINSDIR\egosync-cleanup.ps1"
  ${If} $0 == "error"
    MessageBox MB_OK|MB_ICONSTOP "无法执行安装前清理脚本。请手动关闭 EgoSync 和 opencode.exe 后重试。"
    Abort
  ${EndIf}
  ${If} $0 != 0
    MessageBox MB_OK|MB_ICONSTOP "无法停止旧版 EgoSync 或其 sidecar。请手动关闭 EgoSync 和 opencode.exe 后重试。详细日志：%TEMP%\egosync-install-cleanup.log"
    Abort
  ${EndIf}
  IfFileExists "$INSTDIR\EgoSync.exe" 0 egosync_preinstall_probe_sidecar
  FileOpen $1 "$INSTDIR\EgoSync.exe" w
  IfErrors egosync_preinstall_file_error
  FileClose $1

egosync_preinstall_probe_sidecar:
  IfFileExists "$INSTDIR\resources\opencode.exe" 0 egosync_preinstall_done
  FileOpen $1 "$INSTDIR\resources\opencode.exe" w
  IfErrors egosync_preinstall_file_error
  FileClose $1

egosync_preinstall_done:
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Push $0
  Push $1

  InitPluginsDir
  ClearErrors
  FileOpen $0 "$PLUGINSDIR\egosync-uninstall-cleanup.ps1" w
  IfErrors egosync_preuninstall_script_error
  FileWrite $0 "param([string]$$InstallDir)$\r$\n"
  FileWrite $0 "$$ErrorActionPreference = 'Stop'$\r$\n"
  FileWrite $0 "$$LogFile = Join-Path $$env:TEMP 'egosync-uninstall-cleanup.log'$\r$\n"
  FileWrite $0 "function Log([string]$$Msg) { Add-Content -Path $$LogFile -Value $$Msg -ErrorAction SilentlyContinue }$\r$\n"
  FileWrite $0 "try { if (-not (Test-Path -LiteralPath $$InstallDir)) { exit 0 }; $$root = (Resolve-Path -LiteralPath $$InstallDir -ErrorAction Stop).Path.TrimEnd('\'); $$targets = @((Join-Path $$root 'EgoSync.exe'), (Join-Path $$root 'resources\opencode.exe')); $$norm = $$targets | ForEach-Object { [IO.Path]::GetFullPath($$_).ToLowerInvariant() }; $$procs = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { $$_.ExecutablePath -and $$norm.Contains(([IO.Path]::GetFullPath($$_.ExecutablePath).ToLowerInvariant())) }); foreach ($$p in $$procs) { Stop-Process -Id $$p.ProcessId -Force -ErrorAction SilentlyContinue }; Start-Sleep -Seconds 1; $$alive = @(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object { ($$procs.ProcessId -contains $$_.ProcessId) -and $$norm.Contains(([IO.Path]::GetFullPath($$_.ExecutablePath).ToLowerInvariant())) }); if ($$alive.Count -gt 0) { throw ('Processes still alive: ' + ($$alive.ProcessId -join ', ')) }; exit 0 } catch { Log ('FAILED: ' + $$_); exit 1 }$\r$\n"
  FileClose $0
  Goto egosync_preuninstall_execute

egosync_preuninstall_script_error:
  MessageBox MB_OK|MB_ICONSTOP "无法创建卸载清理脚本。"
  Abort

egosync_preuninstall_execute:
  nsExec::ExecToLog 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\egosync-uninstall-cleanup.ps1" "$INSTDIR"'
  Pop $0
  Delete "$PLUGINSDIR\egosync-uninstall-cleanup.ps1"
  ${If} $0 == "error"
    MessageBox MB_OK|MB_ICONSTOP "无法执行卸载清理脚本。请手动关闭 EgoSync 和 opencode.exe 后重试。"
    Abort
  ${EndIf}
  ${If} $0 != 0
    MessageBox MB_OK|MB_ICONSTOP "卸载前仍有 EgoSync 或 sidecar 进程运行。详细日志：%TEMP%\egosync-uninstall-cleanup.log"
    Abort
  ${EndIf}
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
