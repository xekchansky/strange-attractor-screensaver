# Правила разработки проекта: strange-attractor-screensaver

> Стек: `.appex`/`.saver`, SceneKit, собственный парсер формул, Swift, без внешних зависимостей (исключения одобрены: `swift-testing` для тест-таргета, xcodegen для генерации проекта хоста).
> Среда: macOS 26 arm64. С 2026-09-30 установлен **полный Xcode 27.0** (`xcodebuild`/SDK macOS 27) — хост собирается локально.

## Локальный запуск

### Чистое ядро (SPM)
Прогон тестов (ядро): см. `mem:build_environment` — точная команда с литеральным путём к `usr/lib`. Выборочно: `swift test --filter <Symbol>`.

### Визуализация (демо, AppKit + SceneKit)
- SPM-пакеты `Render/` (`AttractorRender`) и `Demo/` (исполняемый `AttractorDemo`, зависит от `Render` и `Core`). Сборка: `cd Demo && swift build`.
- Аргументы: `--attractor <id>`, `--param name=value` (или голые `name=value`); оверрайды `--trail-length`, `--speed`, `--step`, `--cam-speed`, `--trail-color-speed`; без аргументов — случайный выбор.

### Хост-заставка (M5) — собирается в этой среде, установка `.saver`
- Xcode-проект генерируется **xcodegen** из `Host/project.yml`. Таргеты: `AttractorSaver` (`.saver`, **установочный**), `AttractorScreensaver` (`.appex`, App Store), `AttractorHostApp` (`.app`-контейнер).
- Сборка установочного: `cd Host && xcodegen generate && xcodebuild -project AttractorHost.xcodeproj -scheme AttractorSaver -configuration Debug build`.
- **Песочница:** `xcodebuild` с локальными SPM-пакетами падает на `sandbox_apply: Operation not permitted` (Resolve Package Graph) → сборку хоста выполняем ВНЕ песочницы (терминал пользователя / префикс `!`).
- Установка: `cp -R .../Build/Products/Debug/AttractorSaver.saver ~/Library/Screen\ Savers/`, затем `killall WallpaperAgent`. Проверка в System Settings → Wallpaper → Screen Saver (сделано, работает).
- **Ключевой вывод:** для ручной установки macOS загружает классический `.saver`-бандл (`CFBundlePackageType=BNDL`, `NSPrincipalClass`); расширения `.appex` (`NSExtensionPointIdentifier=com.apple.screensaver`) подхватываются только из приложений App Store. Оба используют один `ScreensaverView`.

## Тестирование (стратегия)

Правило: **ядро тестируется автоматически, рендер/хост — вручную.**
1. **Юнит-тесты ядра (SPM, `swift test`, Swift Testing, 63 теста)** — декодинг, парсер, интегрирование (RK4/Euler), модель/движок, выбор (диапазоны и seed), здоровье симуляции (залипание/цикл/разлёт/NaN и калибровка на Chua).
2. **Визуальная проверка** (демо, вручную).
3. **Смоук на реальном скринсейвере** (`.saver`, вручную): Preview/Test в System Settings, старт/стоп, случайность, формулы в углу.

## Код-стайл и конвенции
- Код — английский (идентификаторы, комментарии); README/memories — русский.
- Модульность: чистое ядро (`AttractorCore`) / рендер (`AttractorRender`, общий для демо и хоста) / хост (`.saver`/`.appex`).
- Тесты рядом с кодом; публичные API — с `///`; без внешних зависимостей без необходимости.
- Naming: типы-существительные, методы-глаголы.

## Актуальные ограничения среды (обновлено 2026-09-30)
- **LSP недоступен** (`Active language servers: []`): символьные write-инструменты Serena не работают; новые файлы — `create_text_file`, существующие — нативный точечный `Edit`.
  - **Осторожно при копировании Swift-файлов:** вставка может удвоить экранирование (`\\(`), ломая интерполяцию `"\(...)"` (должен быть одинарный бэкслеш).
- **Песочница и `xcodebuild`:** резолвинг локальных SPM-пакетов падает на `sandbox_apply` → сборку хоста выполнять вне песочницы.
- **Установка заставок:** работает `.saver` в `~/Library/Screen Savers/`; `.appex` — только App Store.
- **Тригонометрия в формулах — РЕШЕНО (M2):** собственный `FormulaParser`.

## Git-практика
- Атомарные коммиты, сообщения по смыслу (тип: описание), на английском; ветки `feature/mN-*`, мёрдж после зелёных тестов; `swift test` перед коммитом ядра. `.xcodeproj` коммитится (перегенерируем по `project.yml`).

## Итеративный чеклист (майлстоуны)
- **M0 — Фундамент.** ✅ SPM-пакет ядра; тесты; git-практика.
- **M1 — Модель данных и конфиг.** ✅ Дескриптор/Параметр/MinMax/гиперпараметры; `attractors.json`; `AttractorCatalog`; тесты декодинга.
- **M2 — Математическое ядро.** ✅ State/Derivatives, FormulaParser+Expression, AttractorModel, ODESolver (RK4+Euler), AttractorEngine; численные тесты.
- **M3 — Выбор.** ✅ SplitMix64, SimulationSettings, AttractorSelection, StateRange(+initialState), Randomizer, DebugConfig.
- **M4 — Визуализация (демо).** ✅ `Demo/` (`AttractorDemo`), общий `AttractorRender` (`Render/`): TraceRenderer, CameraController, SimulationScene; CLI.
- **M5 — Хост-заставка.** ✅ Таргеты `Host/` (xcodegen): `ScreensaverView`, переиспользуемый `AttractorRender`, формулы в углу; `.saver` собран, **установлен и проверен в System Settings**; `.appex`/`.app` — App Store-путь.
- **M6 — Полировка.** Производительность, крайние случаи, упаковка.

Статус отмечаем в README (раздел «Дорожная карта»).
