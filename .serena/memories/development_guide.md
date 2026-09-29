# Правила разработки проекта: strange-attractor-screensaver

> Стек утверждён: `.appex`, SceneKit, собственный парсер формул, Swift, без внешних зависимостей (исключения одобрены: `swift-testing` для тест-таргета, xcodegen для генерации проекта хоста).
> Среда разработки: macOS 26 arm64. С 2026-09-30 установлен **полный Xcode 27.0** (`xcodebuild`/SDK macOS 27 доступны) — `.appex` собирается локально; до этого был только Command Line Tools.

## Локальный запуск

### Чистое ядро (SPM) — работает из папки проекта
Прогон тестов (ядро): см. `mem:build_environment` — точная рабочая команда с литеральным путём к `usr/lib`.
Выборочно: `swift test --filter <Symbol>`.

### Визуализация (демо-приложение, AppKit + SceneKit)
- SPM-пакеты `Render/` (`AttractorRender`) и `Demo/` (исполняемый `AttractorDemo`, зависит от `Render` и `Core` по локальному пути). Сборка из CLI: `cd Demo && swift build`.
- Аргументы: `--attractor <id>`, `--param name=value` (или голые `name=value`); без аргументов — случайный выбор. Оверрайды: `--trail-length`, `--speed`, `--step`, `--cam-speed`, `--trail-color-speed`.
- Общий рендер (SimulationScene/TraceRenderer/CameraController) живёт в `AttractorRender`; и демо, и хост используют одну копию.

### Хост-скринсейвер (`.appex`, M5) — собирается в этой среде
- Xcode-проект генерируется **xcodegen** из `Host/project.yml`; таргет Screen Saver `AttractorScreensaver` (`.appex`), локальные SPM-пакеты `Core`/`Render`.
- Сборка: `cd Host && xcodegen generate && xcodebuild -project AttractorHost.xcodeproj -scheme AttractorScreensaver -configuration Debug build`.
- **ВАЖНО про песочницу сессии:** `xcodebuild` с локальными SPM-пакетами падает на `sandbox-exec: sandbox_apply: Operation not permitted` (Resolve Package Graph). Поэтому команду сборки хоста выполняем ВНЕ песочницы (в терминале пользователя, либо через префикс `!` в сессии).
- Установка в System Settings (~/Library/Screen Savers/ или через System Settings) и ручная проверка (предпросмотр, старт/стоп, случайность, формулы в углу) — ручной этап на машине с Xcode.

## Тестирование (стратегия)

Правило: **ядро тестируется автоматически, рендер/хост — вручную.**

1. **Юнит-тесты ядра (SPM, `swift test`, Swift Testing)** — основной уровень: декодинг конфига, парсер формул, численное интегрирование (RK4/Euler), модель/движок, выбор (диапазоны и seed).
2. **Визуальная проверка** (демо-приложение, вручную): картинка траектории, вращение камеры, смена цвета следа, формулы/параметры в углу.
3. **Смоук на реальном скринсейвере** (`.appex`, вручную): Preview и Test в System Settings, старт/остановка, случайность запусков.

Перед коммитом обязателен зелёный `swift test` (для шагов, затрагивающих ядро).

## Код-стайл и конвенции
- **Язык кода:** Swift. Идентификаторы и комментарии в коде — на английском.
- **Документация и общение:** README, memories — на русском.
- **Модульность:** чистое ядро (SPM `AttractorCore`) отдельно от рендера (SPM `AttractorRender`) и хоста (`.appex`). Рендер — общий для демо и хоста, без дублирования.
- **Тесты рядом с кодом:** для каждого публичного символа ядра есть покрытие.
- **Публичные API** — с `///`-комментариями.
- **Зависимости:** не добавляем внешние без необходимости; предпочтение Apple-фреймворкам (одобренные исключения см. выше).
- **Naming:** типы — существительные (`ODESolver`, `TraceRenderer`), функции/методы — глаголы (`step`, `select`, `render`).

## Актуальные ограничения среды (обновлено 2026-09-30)
- **LSP недоступен** (`Active language servers: []`): символьные/файловые write-инструменты Serena (`replace_content`, `replace_in_files`, `replace_symbol_body`, ...) на существующих Swift-файлах возвращают `No language servers available` / `Cannot extract symbols`.
  - Новые файлы создаём `create_text_file` (не требует LSP) — работает.
  - Правку существующих файлов (Swift, README, `Package.swift`, `project.yml`, JSON) делаем точечным нативным `Edit` с точным совпадением (не переписываем целиком), помечая это в резюме.
  - **Отдельная осторожность при копировании Swift-файлов:** переданная строка вставляет `\\(` и другие экранирования «дословно» — проверять интерполяцию строк (`"\(...)"` должен быть с ОДИНАРНЫМ бэкслешем).
- **Песочница и `xcodebuild`:** резолвинг локальных SPM-пакетов в `xcodebuild` падает на `sandbox_apply: Operation not permitted` → сборку хоста выполнять вне песочницы (терминал/`!`).
- **Тригонометрия в формулах — РЕШЕНО (M2).** Собственный `FormulaParser` → AST `Expression` с `sin/cos/tan/sqrt/abs/exp/ln/log`. Конфиг `attractors.json` не меняется.
- **`swift test` в среде:** требуется литеральный путь к `usr/lib` (см. `mem:build_environment`); при чистке scratch-пути долго пересобирается `swift-testing` — это нормально.

## Git-практика
- Атомарные коммиты: один логический шаг = один коммит, сообщение по смыслу (тип: описание), на английском.
- Каждый майлстоун завершается зелёными тестами и (если нужно) отдельным коммитом.
- Работа в ветках по этапам (`feature/mN-*`), мёрдж в `main` после зелёных тестов.
- `swift test` перед коммитом ядра. `.xcodeproj` из xcodegen, как правило, коммитится (перегенерируем по `project.yml`).

## Итеративный чеклист (майлстоуны)
- **M0 — Фундамент.** ✅ SPM-пакет ядра; тесты поднимаются; git-практика.
- **M1 — Модель данных и конфиг.** ✅ `AttractorDescriptor`, `Parameter`, `MinMax`, `SimulationHyperparameters`; `attractors.json`; `AttractorCatalog`; тесты декодинга.
- **M2 — Математическое ядро.** ✅ `State`/`Derivatives`, `FormulaParser`+`Expression`, `AttractorModel`, `ODESolver` (RK4+Euler), `AttractorEngine`; численные тесты.
- **M3 — Выбор.** ✅ `SplitMix64`, `SimulationSettings`, `AttractorSelection`, `StateRange`(+`initialState`), `Randomizer`, `DebugConfig`; тесты диапазонов и seed.
- **M4 — Визуализация (демо).** ✅ SPM-пакет `Demo/` (`AttractorDemo`), общий рендер-модуль `AttractorRender` (`Render/`): `TraceRenderer`, `CameraController`, `SimulationScene`; CLI `--attractor`/`--param`; `swift build` в `Demo/`.
- **M5 — Хост `.appex`.** 🟡 Таргет Screen Saver (`Host/`, xcodegen): `ScreensaverView`, переиспользуемый `AttractorRender`, формулы/параметры в углу; сборка `xcodebuild` зелёная; установка/проверка в System Settings — ручной этап.
- **M6 — Полировка.** Производительность, крайние случаи, упаковка и документация.

Статус отмечаем в README (раздел «Дорожная карта»).
