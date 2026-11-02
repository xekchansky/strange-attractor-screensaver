# Архитектура: strange-attractor-screensaver

macOS-скринсейвер, который рендерит странные аттракторы (детерминированные хаотические системы) в 3D. При запуске случайно выбирается аттрактор, его параметры и гиперпараметры симуляции; траектория визуализируется с вращающейся камерой.

> Даты: анализ 2026-09-29, M2 завершён 2026-09-29, M3 завершён 2026-09-29, M4 завершён 2026-09-29, **M5 завершён 2026-09-30** (хост-заставка собрана и установлена/проверена вручную как `.saver`). 2026-11-02 — самолечение неудачных запусков (`SimulationHealthMonitor`, полный случайный перевыбор при сбое). Стек: современный `.appex`-путь остался для App Store, локальная установка идёт классическим `.saver`.

## Принятые ключевые решения (утверждены)
- **Формат скринсейвера:** заявлен современный `.appex`; на практике ручная установка на macOS идёт классическим **`.saver`**-бандлом в `~/Library/Screen Savers/` (расширения `.appex` → `com.apple.screensaver` подхватываются только из приложений App Store). Оба используют один `ScreensaverView`.
- **Рендеринг:** SceneKit (SCNView), траектория — точки/линии, анимируемая камера.
- **Парсинг формул:** собственный рекурсивный парсер `FormulaParser` → AST `Expression` (NSExpression не поддерживает `sin/cos/tan`).
- **Конфиг:** JSON + Codable (без внешних зависимостей).
- **Зависимости:** внешние отсутствуют, только Apple-фреймворки. Исключения: `swift-testing` (тест-таргет), dev-инструмент `xcodegen` (генерация Xcode-проекта).
- **Общий рендер-модуль:** `AttractorRender` — один источник рендер-кода для `Demo/` и хоста (без дублирования).

## Окружение
- macOS 26.6.2, arm64 (Apple Silicon), Swift 6.x.
- **Полный Xcode 27.0 установлен (с 2026-09-30)** — `.appex`/`.saver` собираются локально. Ранее был только Command Line Tools.
- Внешняя песочница сессии: `xcodebuild` с локальными SPM-пакетами падает на `sandbox-exec: sandbox_apply: Operation not permitted` при Resolve Package Graph → сборку хоста выполняем вне песочницы (терминал пользователя или префикс `!`).
- **LSP недоступен**: символьные write-инструменты на существующих Swift-файлах не работают; новые файлы — `create_text_file`, существующие — нативный точечный `Edit`.

## Компоненты

### 1. Общее ядро (SPM-модуль `AttractorCore`) — чистое, тестируемое
- **Config:** `attractors.json` (ресурс бандла) → `[AttractorDescriptor]` через `AttractorCatalog`.
- **Мат. ядро:** `State`, `Derivatives`, `Expression`+`FormulaParser`, `AttractorModel` (`rate(of:)`), `ODESolver` (RK4+Euler), `AttractorEngine` (шаг + кольцевой буфер `trace`).
- **Выбор:** `SplitMix64`, `SimulationSettings`, `AttractorSelection` (`buildModel()`), `StateRange`/`initialState`, `Randomizer`, `DebugConfig`.
- **Самолечение:** `SimulationHealthMonitor` (+ `SimulationHealthIssue`, `SimulationHealthThresholds`) — поток состояний → вердикт (`nonFinite`/`diverged`/`collapsed`/`periodic`), чистое ядро, детерминированное расписание (часто в начале, реже дальше).

### 2. Рендер (SPM-модуль `AttractorRender`, AppKit+SceneKit)
- `TraceRenderer` — полилиния трейса с градиентом (`TraceRenderer.geometry(for:colorOffset:)`).
- `CameraController` — орбита + автокадрирование (`update(dt:rotationSpeed:around:cameraNode:)`).
- `SimulationScene` — @MainActor, `public init(engine:selection:healthMonitor:)` (монитор опционален), `start()`, `stop()`, `public let view: SCNView`, `public let selection`; свой Timer (1/60s), шагает движок, кормит монитор, при сбое один раз вызывает `onNeedsRestart` и останавливается; рисует оверлей формул/параметров/позиции в углу.

### 3. Демо (SPM-пакет `Demo/`, исполняемый `AttractorDemo`)
- `main.swift` + `AppDelegate` + `CommandLineArguments`; рендер через `import AttractorRender`. CLI `--attractor`/`--param`/оверрайды; без аргументов — случайный выбор.

### 4. Хост/оболочка (`Host/`, Xcode-проект через xcodegen)
- **`AttractorSaver`** (`.saver`, `type: bundle`, `WRAPPER_EXTENSION=saver`) — классический бандл, который macOS загружает для локальных заставок: `CFBundlePackageType=BNDL`, `NSPrincipalClass=$(PRODUCT_MODULE_NAME).ScreensaverView`, статически линкует `AttractorCore`+`AttractorRender`. **Это рабочий способ установки.**
- `AttractorScreensaver` (`.appex`, `NSExtension` → `NSExtensionPointIdentifier=com.apple.screensaver`, `NSExtensionPrincipalClass=...ScreensaverView`) — современный app-extension для App Store; System Settings вручную его игнорирует.
- `AttractorHostApp` (`.app`) — приложение-контейнер с `Contents/PlugIns/AttractorScreensaver.appex` (тоже App Store-путь).
- `Host/Sources/ScreensaverView.swift` — `final class ScreensaverView: ScreenSaver.ScreenSaverView`: на `startAnimation()` грузит каталог, строит выбор через `Randomizer`, `buildModel()`+`AttractorEngine`, создаёт/встраивает `SimulationScene` с `SimulationHealthMonitor`, `scene.start()`; на `stopAnimation()` — `scene.stop()`. При `onNeedsRestart` — `tearDownScene()` + полный новый случайный выбор (колбэк `[weak self]`, без retain-цикла). Демо лечится только без аргументов. Инициализаторы не переопределяются (наследуются от NSView).
- Сборка: `cd Host && xcodegen generate && xcodebuild -project AttractorHost.xcodeproj -scheme AttractorSaver -configuration Debug build` (вне песочницы).
- Установка: `cp -R .../AttractorSaver.saver ~/Library/Screen\ Savers/`, `killall WallpaperAgent`; проверка в System Settings → Wallpaper → Screen Saver. (Сделано, работает.)

## Точка входа и поток данных
- **Заставка:** `ScreensaverView` (startAnimation) → `AttractorCatalog.bundled()` → `Randomizer.select(from:)` → `buildModel()` → `AttractorEngine` → `SimulationScene` → рендер трейса/камеры/оверлей до `stopAnimation()`.
- **Дебаг:** `Demo` с CLI-аргументами или случайный выбор.
- Данные: `attractors.json` → дескрипторы → `AttractorModel` → поток точек → `TraceRenderer` → экран.

## Сборка / проект
- `Core/` — SPM, `swift test` (Swift Testing, 63 теста). `Render/`, `Demo/` — SPM, `swift build`.
- `Host/` — Xcode-проект (xcodegen) с таргетами `.saver` (установочный), `.appex`/`.app` (App Store).
