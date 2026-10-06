; Hook cho installer NSIS của Tauri (`bundle.windows.nsis.installerHooks` trong tauri.conf.json).
; Các macro NSIS_HOOK_* được chèn vào installer.nsi mà Tauri sinh ra, dùng được ${BUNDLEID},
; ${PRODUCTNAME}, ${MAINBINARYNAME}, $INSTDIR của file đó.

!define TINYWORLD_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

; Bấm đúp bộ cài mà máy đang có bản cũ hơn: chạy lại chính bộ cài như lúc cập nhật trong app (update.rs,
; `/P /UPDATE /R`): cài đè vào đúng thư mục cũ, giữ pet và cài đặt, xong tự mở lại. Không thì Tauri hiện
; trang "Already Installed" chọn sẵn gỡ bản cũ trước: có máy gỡ không được ("Unable to uninstall!"), gỡ được
; thì cũng mất "Chạy cùng Windows".
; Cài lại cùng bản hay về bản cũ hơn vẫn qua trang đó như thường. File này được chèn lên đầu installer.nsi,
; trước ${VERSION} và các biến của Tauri, nên version của bộ cài đọc từ chính file .exe.
!define MUI_CUSTOMFUNCTION_GUIINIT TinyWorldGuiInit
Function TinyWorldGuiInit
  Push $0
  Push $1
  Push $2
  Push $3
  ; Đang chạy passive (bản cập nhật trong app, hay /P) thì không có trang hỏi gỡ: để yên.
  ${GetOptions} $CMDLINE "/UPDATE" $0
  ${If} ${Errors}
    ${GetOptions} $CMDLINE "/P" $0
  ${EndIf}
  ${If} ${Errors}
    ReadRegStr $0 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\$(^Name)" "DisplayVersion"
    ReadRegStr $1 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\$(^Name)" "UninstallString"
    ${If} $0 != ""
    ${AndIf} $1 != ""
      GetDLLVersion "$EXEPATH" $1 $2
      IntOp $3 $1 >> 16
      IntOp $1 $1 & 0xFFFF
      IntOp $2 $2 >> 16
      ${VersionCompare} "$3.$1.$2" $0 $1
      ${If} $1 = 1
        ; /D= (nếu có) phải đứng cuối nên cờ của mình đặt trước.
        ${GetParameters} $2
        ClearErrors
        Exec '"$EXEPATH" /P /UPDATE /R $2'
        ${IfNot} ${Errors}
          Quit
        ${EndIf}
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

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
