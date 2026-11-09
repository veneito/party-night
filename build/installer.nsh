; Extra installer steps for Party Night (included by electron-builder's NSIS installer).
; Phones on the same Wi-Fi must reach the game server, so let it through Windows Firewall at install time
; and remove those rules again on uninstall.
; - Deleting by name first also clears any BLOCK rule Windows saved if someone pressed Cancel on the
;   firewall popup (Windows names those rules after the app, "Party Night").
; - All network profiles: Windows often marks home Wi-Fi as "Public", and a private-only rule silently fails there.
;   The server only listens while the app is open.

!macro customInstall
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Party Night"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Party Night" dir=in action=allow program="$INSTDIR\Party Night.exe" enable=yes profile=any'
  nsExec::Exec 'netsh advfirewall firewall add rule name="Party Night" dir=in action=allow protocol=TCP localport=3000-3020 enable=yes profile=any'
!macroend

!macro customUnInstall
  nsExec::Exec 'netsh advfirewall firewall delete rule name="Party Night"'
!macroend
