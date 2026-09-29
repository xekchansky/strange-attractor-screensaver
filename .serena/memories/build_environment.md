# Сборка и тесты ядра в этой среде (CLI-only, CommandLineTools без Xcode)

Актуально для `Core/` (SPM-пакет `AttractorCore`). См. `mem:development_guide`.

## Swift Testing вместо XCTest (одобренное отклонение)
- В SDK нет `XCTest` → тесты на **Swift Testing** (`import Testing`, `@Test`/`#expect`).
- Зависимость `swift-testing` в `Core/Package.swift` по ревизии `48d727cc1cf4eda667c858c501495f1018f69d21` (= `swift-6.3.3-RELEASE`). Сознательное исключение из «без внешних зависимостей» (только тест-таргет), подтверждено пользователем.

## Песочница сессии (важно)
- В `.claude/settings.local.json` добавлен `sandbox.filesystem.allowWrite` на `/var/folders/**`, `~/Library/org.swift.swiftpm`, `~/Library/Caches/org.swift.swiftpm`, `~/.cache/org.swift.swiftpm`.
- **Внешнюю песочницу и файлы её политики не менять** — только точечный allowWrite выше (согласовано).

## Рабочая команда сборки/тестов ядра (из Core/)
```bash
swift test --disable-sandbox --scratch-path "$TMPDIR/attractor-scratch" \
  -Xlinker -L -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib \
  -Xlinker -rpath -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib
```
Почему так: `--disable-sandbox` (иначе вложенная SPM-песочница `Package.swift` падает на `sandbox_apply`), `--scratch-path "$TMPDIR/..."` (иначе `.build/checkouts/*/.git` не пишется), `-Xlinker -L/-rpath .../usr/lib` (там `lib_TestingInterop.dylib`, нужная Swift Testing). **Путь к `usr/lib` — литералом**, не через инлайн-префикс `LIBCACHE=...` (не подставляется → пустые `-L`/`-rpath`). `swift build` (без тестов) зелёный и без флагов.

## Сборка демо и рендера (SPM, из той же среды)
```bash
cd Demo && swift build --disable-sandbox --scratch-path "$TMPDIR/attractor-demo-scratch" \
  -Xlinker -L -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib \
  -Xlinker -rpath -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib
```
(тот же приём, что и ядро; демо компилирует `AttractorRender` + `AttractorCore` по локальным путям).

## Сборка хоста `.appex` (Xcode 27, зафиксировано 2026-09-30)
- Проект генерирует **xcodegen**: `brew install xcodegen`; `cd Host && xcodegen generate`.
- Сборка: `cd Host && xcodebuild -project AttractorHost.xcodeproj -scheme AttractorScreensaver -configuration Debug -derivedDataPath /tmp/host-dd build`.
- **Критично:** `xcodebuild` с локальными SPM-пакетами (`Core`/`Render`) падает в песочнице сессии на `sandbox-exec: sandbox_apply: Operation not permitted` при `Resolve Package Graph`. Выполнять команду ВНЕ песочницы (в терминале пользователя или через префикс `!` в сессии).
- Продукт: `.../Debug/AttractorScreensaver.appex`; ресурс-каталог ядра встроен как `Contents/Resources/AttractorCore_AttractorCore.bundle`.

Предупреждение незначительное: macOS-деплой 13.0 против dylib из SDK — не блокирует.
