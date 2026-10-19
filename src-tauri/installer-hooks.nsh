!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHCTX "Software\Classes\ReportStudio.Project\DefaultIcon" "" `"$INSTDIR\rproj.ico",0`
  !insertmacro UPDATEFILEASSOC
!macroend
