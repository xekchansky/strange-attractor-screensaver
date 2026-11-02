# Установка `.saver` на macOS: только заменой бандла, не поверх

## Симптом
После обновления `AttractorSaver.saver` превью падает: `Extension process ... exited`, в crash-логах `EXC_BAD_ACCESS`, `SIGKILL (Code Signature Invalid)`, `termination CODESIGNING / Invalid Page` в стеке `SSPreflightModuleLoadForPath` → `NSBundle preflightAndReturnError` → `dlopen_preflight`. При этом `codesign --verify --deep --strict` проходит, а SHA-256 установленного и свежесобранного бинарников совпадают — «забыли подписать» здесь ни при чём.

## Причина (2026-11-03)
Бандл обновили копированием **поверх** существующего каталога: `Contents/MacOS/AttractorSaver` перезаписался в **том же inode**, который уже был отображён в память работавшим `legacyScreenSaver`. Ядро/AMFI держат закэшированное code-signing состояние vnode, и новые страницы файла не проходят проверку → `Invalid Page`. Байт-идентичная копия того же файла на новом inode проходит `dlopen_preflight` (`true`).

Признак на диске: mtime бинарника новее mtime каталога `.saver` (каталог от старой установки, файл — от новой).

## Диагностика
- `stat -f '%i %z %Sm'` на установленном и собранном бинарниках — сравнить inode и mtime.
- `dlopen_preflight` через python ctypes: на «отравленном» inode процесс получает SIGKILL (exit 137), на `cp`-копии возвращает `true`.
- В crash-логе проверить, что faulting-адрес лежит внутри `__TEXT` нашего бинарника (символы `AttractorSaver`), а не системного фреймворка.

## Правило установки
```sh
rm -rf "$HOME/Library/Screen Savers/AttractorSaver.saver"
ditto <build>/AttractorSaver.saver "$HOME/Library/Screen Savers/AttractorSaver.saver"
codesign --verify --deep --strict --verbose=4 "$HOME/Library/Screen Savers/AttractorSaver.saver"
```
Никогда не копировать внутрь существующего `.saver` (`cp -R` мержит и перезаписывает файлы на месте) и не перезаписывать бандл, пока его модуль мог быть загружен `legacyScreenSaver`/`WallpaperAgent`.

Связано: `mem:macos_input_lockup`, `mem:development_guide`.
