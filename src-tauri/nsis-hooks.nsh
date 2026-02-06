; Voxai NSIS installer hooks
; Clean up user data directories on uninstall

!macro NSIS_HOOK_POSTUNINSTALL
  ; Remove config and history (%APPDATA%\Voxai)
  RMDir /r "$APPDATA\Voxai"
  ; Remove models and cache (%LOCALAPPDATA%\Voxai)
  RMDir /r "$LOCALAPPDATA\Voxai"
!macroend
