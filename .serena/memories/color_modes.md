# Цветовые режимы трейла (M6, итерация 4)

## Решение
`TraceRenderer` красит трейл одним из четырёх режимов — `TraceColorMode`:
- `.trail` — радужный градиент по положению точки вдоль трейла (прежнее поведение).
- `.age` — ч/б затемнение: старые точки темнее, новые ярче.
- `.depth` — цвет по расстоянию точки от камеры (глубина).
- `.spatial` — цвет по координатам точки: x→R, y→G, z→B, нормализованные по трейлу.

## Как выбирается
- Режим живёт в `SimulationSettings.colorMode` (тип `TraceColorMode: String, CaseIterable, Codable, Sendable` — в ядре `AttractorCore`).
- `Randomizer.select(from:using:)` случайно выбирает режим через `TraceColorMode.allCases.randomElement(using:)` (детерминированно по seed → работает и в скринсейвере, и в демо).
- `DebugConfig`/`SimulationSettings(midpointsOf:)` по умолчанию дают `.trail`.
- Оверрайд в демо: CLI `--color-mode <trail|age|depth|spatial>` (`CommandLineArguments`).

## Рендер
- `TraceRenderer.geometry(for:colorOffset:mode:cameraPosition:)` — контекст камеры нужен только для `.depth`; статистика (min/max расстояний или per-axis bounds) собирается в первом проходе по positions, цвет считается во втором. `colorOffset` (`trailColorSpeed`) сдвигает гамму в `trail`/`age`/`depth`. В `spatial` offset не применяется (цвет статичен по координатам): циклический сдвиг всех трёх каналов давал резкое «стробящее» переливание (каналы обрывались 1→0 в разное время), поэтому координатный цвет оставлен стабильным.
- `SimulationScene.tick` вызывает `cameraController.update` ДО построения геометрии, чтобы позиция камеры была актуальна для `.depth`.

## Проверка
- 53 юнит-теста ядра зелёные (добавлены `TraceColorModeTests`, тест Randomizer на валидный режим). `Demo` собирается. Хост `.saver` получает режим автоматически через `Randomizer`/`SimulationScene` (пересборка хоста вне песочницы — на машине с Xcode/терминале).
