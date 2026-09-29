# Архитектура: strange-attractor-screensaver

macOS-скринсейвер, который рендерит странные аттракторы (детерминированные хаотические системы) в 3D. При запуске случайно выбирается аттрактор, его параметры и гиперпараметры симуляции; траектория визуализируется с вращающейся камерой.

> Даты: анализ 2026-09-29, M2 завершён 2026-09-29, M3 завершён 2026-09-29, M4 завершён 2026-09-29, M5 в работе 2026-09-30 (сборка `.appex` зелёная, установка в System Settings ручная). Стек утверждён пользователем: современный `.appex`, SceneKit, собственный парсер формул.

## Принятые ключевые решения (утверждены)
- **Формат скринсейвера:** современный `.appex` (app-extension, macOS 10.15+). Не легаси `.saver`.
- **Рендеринг:** SceneKit (SCNView), траектория — точки/линии, анимируемая камера.
- **Парсинг формул:** собственный рекурсивный парсер `FormulaParser` → AST `Expression` (стандартный `NSExpression` НЕ поддерживает `sin/cos/tan`, а аттрактор Томаса их использует; свой парсер даёт полный контроль и тестируемость в SPM-ядре).
- **Конфиг:** JSON + Codable (без внешних зависимостей).
- **Зависимости:** внешние отсутствуют, только Apple-фреймворки. Исключения: `swift-testing` (только тест-таргет, одобрено), dev-инструмент `xcodegen` (генерация Xcode-проекта хоста, не рантайм-зависимость, одобрено на M5).
- **Общий рендер-модуль:** рендер вынесен в отдельный SPM-пакет `Render/` (`AttractorRender`), на который ссылаются и `Demo/`, и хост `.appex` — один источник кода, без дублирования.

## Окружение
- macOS 26.6.2, arm64 (Apple Silicon), Swift 6.x.
- **Полный Xcode 27.0 установлен (с 2026-09-30)** — `xcodebuild`/SDK macOS 27 доступны, `.appex` собирается локально. Ранее был только Command Line Tools.
- Внешняя песочница сессии: `xcodebuild` с локальными SPM-пакетами падает на `sandbox-exec: sandbox_apply: Operation not permitted` при Resolve Package Graph → команду сборки хоста выполняем вне песочницы (в терминале пользователя или через префикс `!`).
- **LSP недоступен** (см. `mem:development_guide`): символьные write-инструменты на существующих Swift-файлах не работают; новые файлы создаём `create_text_file`, существующие правим нативным точечным `Edit`.

## Компоненты

### 1. Общее ядро (SPM-модуль) — чистое, тестируемое
- **Config-слой:** `attractors.json` (ресурс в бандле) → `[AttractorDescriptor]` через `AttractorCatalog`.
- **Доменное/математическое ядро:** `State`, `Derivatives`, `Expression`+`FormulaParser`, `AttractorModel` (`rate(of:)`), `ODESolver` (RK4+Euler), `AttractorEngine` (шаг + кольцевой буфер `trace`).
- **Выбор:** `SplitMix64` (seedable), `SimulationSettings`, `AttractorSelection` (+`buildModel()`), `StateRange`/`initialState`, `Randomizer`, `DebugConfig`.

### 2. Рендер (SPM-модуль `Render/`, AppKit+SceneKit)
- `TraceRenderer` — полилиния трейса с цветовым градиентом (чистая функция `TraceRenderer.geometry(for:colorOffset:)`, изначально спроектирована для переиспользования хостом).
- `CameraController` — орбита камеры + автокадрирование по трейсу (`update(dt:rotationSpeed:around:cameraNode:)`).
- `SimulationScene` — @MainActor-класс: `public init(engine:selection:)`, `start()`, `stop()`, `public let view: SCNView`, `public let selection: AttractorSelection`; владеет своим Timer (1/60s), шагает движок, рисует оверлей формул/параметров/позиции в углу.
- Пакет `Render/Package.swift`: `.macOS(.v13)`, зависимость на `AttractorCore` (путь `../Core`), продукт `AttractorRender`.

### 3. Демо (SPM-пакет `Demo/`, исполняемый `AttractorDemo`)
- `main.swift` + `AppDelegate` + `CommandLineArguments`; рендер подключён через `import AttractorRender`. CLI `--attractor`/`--param`/оверрайды гиперпараметров; без аргументов — случайный выбор.

### 4. Хост/оболочка (`.appex`, `Host/`)
- Xcode-проект генерируется **xcodegen** из `Host/project.yml` (таргет Screen Saver `AttractorScreensaver` типа `app-extension`, локальные пакеты `Core`/`Render`).
- `Host/Sources/ScreensaverView.swift` — `final class ScreensaverView: ScreenSaver.ScreenSaverView`: на `startAnimation()` грузит каталог, строит выбор через `Randomizer`, `buildModel()`+`AttractorEngine`, создаёт/встраивает `SimulationScene`, `scene.start()`; на `stopAnimation()` — `scene.stop()`. Инициализаторы не переопределяются (наследуются от NSView; в SDK 27 `init(frame:)` — convenience, `init?(coder:)` — required, свойств без дефолта нет).
- `Info.plist` (генерируется): `NSExtensionPointIdentifier = com.apple.ScreenSaver.Saver`, `NSExtensionPrincipalClass = $(PRODUCT_MODULE_NAME).ScreensaverView`, `CFBundlePackageType = XPC!`.
- Сборка: `cd Host && xcodegen generate && xcodebuild -project AttractorHost.xcodeproj -scheme AttractorScreensaver -configuration Debug build` (вне песочницы). Продукт `AttractorScreensaver.appex`; ресурс ядра встраивается как `AttractorCore_AttractorCore.bundle` в `Contents/Resources`. Локальные SPM-пакеты линкуются статически (без дублирования кода).
- Установка/проверка в System Settings — ручной этап.

## Точка входа и поток данных
- **Скринсейвер:** `ScreensaverView` (startAnimation) → `AttractorCatalog.bundled()` → `Randomizer.select(from:)` → `buildModel()` → `AttractorEngine` → `SimulationScene` → рендер трейса/камеры/оверлей до `stopAnimation()`.
- **Дебаг:** `Demo` с CLI-аргументами или случайный выбор.
- Данные: `attractors.json` → дескрипторы → `AttractorModel` (FormulaParser + RK4/Euler в `AttractorEngine`) → поток точек → `TraceRenderer` → экран.

## Сборка / проект
- `Core/` — SPM, `swift test` (Swift Testing). `Render/`, `Demo/` — SPM, `swift build`.
- `Host/` — Xcode-проект (xcodegen) с таргетом `.appex`.
- Юнит-тесты мат. ядра (парсер, точность интегрирования, модель, движок, выбор) — Swift Testing.
