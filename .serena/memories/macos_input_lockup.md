# macOS 26: зависание глобальных горячих клавиш после предпросмотра заставки

## Симптом
После установки/предпросмотра `AttractorSaver.saver` (особенно после краш-лупа Wallpaper/`legacyScreenSaver` из-за неподписанных `.saver`/битого `.app`) в текущей сессии перестают работать:
- Cmd+Tab (Dock не перехватывает);
- переключение раскладки Ctrl+Space / Globe / Caps Lock (TextInputSwitcher/HIToolbox не перехватывает).

Обычный ввод и модификаторы при этом работают.

## Диагностика (2026-11-03)
- `CGEventSource`/`CGEventSource.keyState` из терминала бесполезны без Input Monitoring: `CGPreflightListenEventAccess() == false`, `AXIsProcessTrusted() == false` — монитор «слепой».
- Локальный `NSEvent.addLocalMonitorForEvents` в тестовом AppKit-окне (без TCC) показал, что `Ctrl+Space` (keyCode 49, flags=[ctrl]) и `Cmd+Tab` (keyCode 48, flags=[cmd]) доставляются **активному приложению** как обычные нажатия. Значит, клавиатура и модификаторы в порядке, а глобальный hotkey/event-tap слой сессии не перехватывает шорткаты.
- Набор `abc` при этом даёт `фис` (активна Russian) — переключение не происходит.
- Перезапуск `legacyScreenSaver`, `TextInputSwitcher`, `TextInputMenuAgent`, `SystemUIServer`, `ControlCenter`, `Dock` не помогает. `killall` из sandbox запрещён (`operation not permitted`), выполнялся пользователем через `!`.
- Помог только **reboot** (2026-11-03): все шорткаты восстановились.

## Причина/контекст
macOS 26.6.2. В `~/Library/Logs/DiagnosticReports` в момент инцидента — пачки `Wallpaper`, `WallpaperAgent`, `WallpaperLegacyExtension`, `legacyScreenSaver` с `EXC_BAD_ACCESS`/`SIGKILL (Code Signature Invalid)` в `SSPreflightModuleLoadForPath` → `ScreenSaverModule moduleWithPath` → `dyld4::dlopen_preflight`. Источник — неподписанные legacy-заставки (`ClassicStarfieldSaver.saver`) и битый `AttractorHost.app`. После их удаления краш-луп прекратился, но сессионное состояние осталось сломанным до перезагрузки.

## Профилактика
- Ставить только ad-hoc подписанный `.saver`; проверять `codesign --verify --deep --strict`.
- Обновлять `.saver` только заменой каталога (`rm -rf` + `ditto`), не поверх существующего: перезапись того же inode даёт stale code-signing state и `Invalid Page` даже при валидной подписи (см. `mem:saver_install`).
- Неподписанные/битые `.saver` и `.app` не держать в `~/Library/Screen Savers` и `/Applications`.
- Не злоупотреблять `killall WallpaperAgent`.
- После предпросмотра полностью закрывать System Settings; проверять `lsappinfo list | grep legacyScreenSaver`.
- При появлении симптомов — сразу reboot/logout, не тратя время на поштучные kill'ы.

Связано: `mem:development_guide`, `mem:build_environment`.
