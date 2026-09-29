# Сборка и тесты ядра в этой среде (CLI-only, CommandLineTools без Xcode)

Актуально для `Core/` (SPM-пакет `AttractorCore`). См. `mem:development_guide`.

## Swift Testing вместо XCTest (одобренное отклонение)
- В SDK CommandLineTools нет ни `XCTest`, ни встроенного `Testing` модуля → стандартный `swift test` на XCTest не собирается.
- Тесты ядра пишутся на **Swift Testing** (`import Testing`, `@Test`/`#expect`).
- Зависимость `swift-testing` закреплена в `Core/Package.swift` по ревизии `48d727cc1cf4eda667c858c501495f1018f69d21` (= тег `swift-6.3.3-RELEASE`). Это **сознательное исключение** из политики «без внешних зависимостей» (только тест-таргет), подтверждённое пользователем; при появлении XCTest в среде можно вернуться к нему.

## Песочница сессии (важно)
- В `.claude/settings.local.json` добавлен `sandbox.filesystem.allowWrite` на `/var/folders/**`, `~/Library/org.swift.swiftpm`, `~/Library/Caches/org.swift.swiftpm`, `~/.cache/org.swift.swiftpm`.
  Без этого системный temp/Cache `/var/folders` не писался → SwiftPM/clang падали (`SwiftShims.pcm`, `output-file-map.json`).
- **Внешнюю песочницу этой сессии и файлы её политики не менять** — только точечный allowWrite выше (уже согласовано).

## Рабочая команда сборки/тестов ядра (из Core/)
```bash
swift test --disable-sandbox --scratch-path "$TMPDIR/attractor-scratch" \
  -Xlinker -L -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib \
  -Xlinker -rpath -Xlinker /Library/Developer/CommandLineTools/Library/Developer/usr/lib
```
Почему именно так:
- `--disable-sandbox` — снимает вложенную SPM-песочницу компиляции `Package.swift` (внешняя песочница блокирует её `sandbox_apply`). Флаг согласован с пользователем.
- `--scratch-path "$TMPDIR/..."` — обязателен: иначе зависимости клонируются в `Core/.build/checkouts/*/.git`, а запись во вложенные `.git` в рабочем дереве песочницей запрещена.
- `-Xlinker -L/-rpath .../Developer/usr/lib` — `lib_TestingInterop.dylib` (нужна Swift Testing) лежит там, но SPM сам не добавляет этот путь в этой среде.
- **ВАЖНО (зафиксировано 2026-09-29): путь к `usr/lib` писать литералом, НЕ через инлайн-префикс `LIBCACHE=... swift ...`** — в этой среде переменная через такой префикс не подставляется: линковщик получает пустые `-L`/`-rpath`, и `swift test` падает с `ld: library '_TestingInterop' not found`. `swift build` (без тестов) при этом зелёный.
- Если `swift test` долго пересобирает `swift-testing` из `checkouts/` — это нормально после чистки scratch; инкрементальная сборка потом быстрая.

Предупреждение незначительное: macOS-деплой 13.0 против dylib, собранной под 14.0 — не блокирует.
