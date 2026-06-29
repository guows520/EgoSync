Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Helper {
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
}
"@
Get-Process -Name "egosync" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Output "PID: $($_.Id) Handle: $($_.MainWindowHandle) Title: '$($_.MainWindowTitle)'"
    if ($_.MainWindowHandle -ne [IntPtr]::Zero) {
        [Win32Helper]::ShowWindow($_.MainWindowHandle, 9)
        [Win32Helper]::SetForegroundWindow($_.MainWindowHandle)
        Write-Output "Activated PID: $($_.Id)"
    }
}
