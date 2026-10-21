; Grouper's NSIS installer hooks (bundle.windows.nsis.installerHooks).
;
; PREUNINSTALL runs at the top of the uninstall section, before any file,
; registry key or shortcut is removed. The template only closes a running
; Grouper after this hook, so the hook closes it first (the same prompt
; the template shows), then runs `grouper.exe --cleanup`. Cleanup unpins
; and deletes the group shortcuts in Start Menu\Programs\Grouper, deletes
; that folder, deletes the group icons in
; %LOCALAPPDATA%\Programs\Grouper\group-icons and removes the "Start with
; Windows" entry.
;
; When the user ticks the template's own "Delete the application data"
; box ($DeleteAppDataCheckboxState), the hook also passes --delete-data,
; which deletes app_state.json and the icon caches (the template then
; removes the same folders again, plus the logs). Cleanup logs its errors
; to grouper.log and always exits 0, so it never blocks the uninstall.
;
; An update ($UpdateMode = 1: the updater, or a newer installer replacing
; this one) keeps everything: shortcuts, pins, sign-in entry and data.

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
    DetailPrint "Removing Grouper's group shortcuts and sign-in entry..."
    ${If} $DeleteAppDataCheckboxState = 1
      ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --cleanup --delete-data'
    ${Else}
      ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --cleanup'
    ${EndIf}
  ${EndIf}
!macroend
