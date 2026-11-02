# Сборка и тесты ядра в этой среде (CLI-only, CommandLineTools без Xcode)

Актуально для `Core/` (SPM-пакет `AttractorCore`). См. `mem:development_guide`.

## Swift Testing вместо XCTest (одобренное отклонение)
- Тесты на **Swift Testing** (`import Testing`, `@Test`/`#expect`); `swift-testing` закреплён в `Core/Package.swift` по ревизии `48d727cc1cf4eda667c858c501495f1018f69d21` (= `swift-6.3.3-RELEASE`). Сознательное исключение (только тест-таргет), подтверждено пользователем.

## Песочница сессии (важно)
- В `.claude/settings.local.json` добавлен `sandbox.filesystem.allowWrite` на `/var/folders/**`, `~/Library/org.swift.swiftpm`, `~/Library/Caches/org.swift.swiftpm`, `~/.cache/org.swift.swiftpm`.
- **Внешнюю песочницу и файлы её политики не менять.**

## Рабочая команда сборки/тестов ядра (из Core/)
```bash
swift test --disable-sandbox --scratch-path "$TMPDIR/attractor-scratch" \
  -Xlinker -L -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib \
  -Xlinker -rpath -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib
```
Почему: `--disable-sandbox` (иначе вложенная SPM-песочница падает на `sandbox_apply`), `--scratch-path "$TMPDIR/..."` (иначе `.build/checkouts/*/.git` не пишется), `-Xlinker -L/-rpath .../usr/lib` (там `lib_TestingInterop.dylib`). **Путь к `usr/lib` — литералом**, не через префикс `LIBCACHE=...` (не подставляется). `swift build` (без тестов) зелёный и без флагов.

## Сборка демо и рендера (SPM)
```bash
cd Demo && swift build --disable-sandbox --scratch-path "$TMPDIR/attractor-demo-scratch" \
  -Xlinker -L -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib \
  -Xlinker -rpath -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib
```

## Сборка и установка хоста-заставки (Xcode 27, зафиксировано 2026-09-30)
- Проект генерирует **xcodegen**: `brew install xcodegen`; `cd Host && xcodegen generate`.
- Сборка установочного `.saver`:
```bash
cd Host && xcodebuild -project AttractorHost.xcodeproj -scheme AttractorSaver \
  -configuration Debug -derivedDataPath /tmp/host-dd build
```
  Продукт: `/tmp/host-dd/Build/Products/Debug/AttractorSaver.saver`.
- **Критично:** `xcodebuild` с локальными SPM-пакетами (`Core`/`Render`) падает в песочнице сессии на `sandbox-exec: sandbox_apply: Operation not permitted` при `Resolve Package Graph` → выполнять команду ВНЕ песочницы (терминал пользователя или префикс `!`).
- Установка:
```bash
mkdir -p ~/Library/Screen\ Savers
# Замена бандла, не копирование поверх: cp -R перезаписывает Mach-O в том же
# inode, и если модуль уже был загружен legacyScreenSaver, ядро отвергает
# новые страницы → Invalid Page при валидной подписи (см. mem:saver_install).
rm -rf ~/Library/Screen\ Savers/AttractorSaver.saver
ditto /tmp/host-dd/Build/Products/Debug/AttractorSaver.saver \
      ~/Library/Screen\ Savers/AttractorSaver.saver
killall WallpaperAgent 2>/dev/null
```
  Затем в System Settings → Wallpaper → Screen Saver появляется «AttractorSaver».
- Прочие таргеты (App Store-путь, вручную их система не показывает): `AttractorScreensaver` (`.appex`), `AttractorHostApp` (`.app`-контейнер).
- Приложение-контейнер и расширение должны иметь согласованные bundle id (id расширения — префикс id приложения), иначе `ValidateEmbeddedBinary` падает на 'Embedded binary's bundle identifier is not prefixed'.
- Правильный extension point для `.appex`-заставки — `com.apple.screensaver` (проверено по `lsregister`), не `com.apple.ScreenSaver.Saver`.

Предупреждение незначительное: macOS-деплой 13.0 против dylib из SDK — не блокирует.
