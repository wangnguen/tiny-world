; Hook cho installer NSIS của Tauri (`bundle.windows.nsis.installerHooks` trong tauri.conf.json).
; Các macro NSIS_HOOK_* được chèn vào installer.nsi mà Tauri sinh ra, dùng được ${BUNDLEID},
; ${PRODUCTNAME}, ${MAINBINARYNAME}, $INSTDIR của file đó.

!define TINYWORLD_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

; Tắt mọi bản TinyWorld đang chạy của user này. Tauri chỉ tự tắt "$INSTDIR\tinyworld.exe", nên bản
; portable cũ (tên file, chỗ để khác) vẫn chạy; bản vừa cài mở lên bị single-instance chặn và tự
; thoát, trên màn hình vẫn là pet cũ. Mỗi bản đang chạy, dù ở đâu, đều có cửa sổ ẩn của
; tauri-plugin-single-instance tên "<identifier>-sic" / "<identifier>-siw", tìm theo đó.
; Tắt cứng như Restart Manager của Tauri: pet chỉ mất tối đa một chu kỳ autosave.
!macro TINYWORLD_CLOSE_RUNNING
  Push $0
  Push $1
  Push $2
  StrCpy $2 0
  ${Do}
    FindWindow $0 "${BUNDLEID}-sic" "${BUNDLEID}-siw"
    ${If} $0 P= 0
      ${Break}
    ${EndIf}
    System::Call 'user32::GetWindowThreadProcessId(p r0, *i .r1)'
    ; PROCESS_TERMINATE | SYNCHRONIZE
    System::Call 'kernel32::OpenProcess(i 0x00100001, i 0, i r1) p .r1'
    ${If} $1 P<> 0
      System::Call 'kernel32::TerminateProcess(p r1, i 1)'
      System::Call 'kernel32::WaitForSingleObject(p r1, i 5000)'
      System::Call 'kernel32::CloseHandle(p r1)'
    ${EndIf}
    ; Không tắt được (ví dụ bản chạy bằng quyền admin) thì thôi, để Tauri hỏi như cũ.
    IntOp $2 $2 + 1
    ${If} $2 >= 5
      ${Break}
    ${EndIf}
  ${Loop}
  Pop $2
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro TINYWORLD_CLOSE_RUNNING
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro TINYWORLD_CLOSE_RUNNING
!macroend

; "Chạy cùng Windows" đang bật (autostart.rs ghi giá trị "TinyWorld" trong Run) thì trỏ về bản vừa
; cài, để lần đăng nhập sau không mở lại bản portable cũ rồi chặn luôn bản mới.
!macro NSIS_HOOK_POSTINSTALL
  Push $0
  ReadRegStr $0 HKCU "${TINYWORLD_RUN_KEY}" "${PRODUCTNAME}"
  ${If} $0 != ""
    WriteRegStr HKCU "${TINYWORLD_RUN_KEY}" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe"'
  ${EndIf}
  Pop $0
!macroend
