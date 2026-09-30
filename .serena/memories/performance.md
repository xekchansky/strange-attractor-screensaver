# Производительность (M6)

Направление M6, итерация 1 (2026-09-30). Цель — снизить per-frame CPU рендера без изменения картинки.

## Наблюдение (масштаб данных)
- `trailLength` — гиперпараметр, у части аттракторов диапазон `[8000, 20000]`; ёмкость движка `AttractorEngine.trailCapacity` = `settings.trailLength` (и в хосте, и в демо).
- Рендер на 60 fps делает по два O(n)-прохода по всему трейлу каждый кадр:
  1. `TraceRenderer.geometry(for:colorOffset:)` — заново строит геометрию (массивы positions/colors/indices, копии `Data`, объекты SceneKit);
  2. `CameraController.framing(for:)` — дважды сканирует трейл (мин/макс + радиус).

## Изменение (итерация 1): троттлинг кадрирования
`CameraController` теперь пересчитывает полный фрейминг (bounds+радиус) не каждый кадр, а каждые `framingStride = 6` кадров (~10 Гц). Причина: цель кадрирования и так сглаживается (`smoothing` lerp, easing), поэтому 10-Гц обновление цели визуально неотличимо от покадрового, но убирает ~5/6 O(n)-сканов. Орбита (`azimuth`), положение камеры и `look(at:)` остаются покадровыми. Хранятся кэши `targetCenter`/`targetDistance`; первый кадр (`frame % 6 == 1`) вычисляет цель обязательно.

Без изменения картинки. Проверено: `swift build` в `Demo/` (компилирует Render) зелёный; тесты ядра (49) не затронуты. **Визуально подтверждено пользователем 2026-09-30** (ручной запуск демо — автокадрирование плавное). Ветка `feature/m6-perf-framing` смержена в `main`.

## Изменение (итерация 2): децимация очень длинного трейла
`TraceRenderer.geometry(for:colorOffset:)`: если `trace.count > budget = 16_384`, полилиния строится с постоянным шагом (stride), каппингуясь на ~16k вершин; градиент считается по пропорции исходного индекса (не меняется); «голова» (самый свежий кадр) всегда включена последней. Трейлы ≤ budget — без изменений. Для 20k → ~10k точек (stride 2). Выигрыш: меньше работа CPU (сборка геометрии) и GPU (рисуемые примитивы). Визуально подтверждено 2026-09-30: на 20k с budget=8192 (stride 3) была лёгкая зубчатость — поэтому бюджет поднят до 16k (stride 2), зубчатость практически исчезла при сохранении ~2-кратного выигрыша на максимуме. Ветка `feature/m6-perf-decimate`.

## Изменение (итерация 3): переиспользование буферов в TraceRenderer
`TraceRenderer` стал состоятельным `final class` (вместо статического `enum`): переиспользует предвыделенные `positions`/`colors`/`indices`/`sampledIndices` между кадрами (`removeAll(keepingCapacity:)`), убирая аллокацию сотен КБ/кадр для длинных трейлов. Вывод геометрии и децимация — без изменений. Call-site: `SimulationScene` держит `private let traceRenderer = TraceRenderer()`. Проверено: `swift build` в `Demo/` зелёный; визуально подтверждено пользователем 2026-09-30 (идентично). Ветка `feature/m6-perf-buffers`.

## Изменение (итерация 4): митигация утечки памяти / фонового CPU рендера
Симптом пользователя: процесс-хост `.saver` `legacyScreenSaver.appex` растёт по памяти при каждом срабатывании заставки (до ~2.6 ГБ) и ест CPU, когда заставка «выключена». Причина: `SimulationScene.tick` пересоздавал `SCNGeometry` + новый `SCNMaterial` + Data-копии **каждый кадр** (60 fps); хост не освобождает часто заменяемую геометрию → накопление. Движок корректен (кольцевой буфер `AttractorEngine.trace`).

Митигация (без Metal, выбрано пользователем; Metal-вариант — «Отложенное»):
- `TraceRenderer`: один persistent `SCNMaterial` (`.constant`) вместо создания per-frame.
- `SimulationScene.tick`: (а) per-frame `autoreleasepool`; (б) полилиния пересобирается **каждые 2 такта** (`renderFrame % 2 == 0`, ~30 Гц) — пофреймовая пересборка `SCNGeometry` давала рваную отрисовку превью, троттлинг по числу точек — дёрганье кончика; (в) **самоостановка по отсутствию окна**: если `view.window == nil` ~1.5 c (`maxTicksWithoutWindow = 90`), `SimulationScene` сам зовёт `stop()` (обнуляет `isRunning`). Причина: превью-хост (`WallpaperLegacyExtension`) НЕ зовёт `stopAnimation()` при закрытии окна, и иначе таймер вечно держит сцену/SCNView → фоновый CPU + рост памяти с каждым запуском превью. Активное превью/показ имеют window → не останавливаются. `ScreensaverView.startAnimation` пересоздаёт сцену, если `isRunning == false`. (Мгновенный guard на `isVisible`/`window==nil` давал чёрный/мигающий экран → задержка 1.5 c снимает риск.)
- **Код-подпись `.saver`:** unsigned-бандл (был `CODE_SIGNING_ALLOWED: NO`, linker-signed) **крашит превью** настроек обоев: `WallpaperLegacyExtension` падает `SIGKILL (Code Signature Invalid)`/`Invalid Page` на `dlopen_preflight`. В `Host/project.yml` для `AttractorSaver` включено `CODE_SIGN_IDENTITY: "-"` (ad-hoc), чтобы сборка давала валидную подпись. Ручной фикс: `codesign --force --sign - <saver>`.
- Вывод/цвета не изменены. Ветка `feature/m6-perf-mitigate` — проверить в Activity Monitor за `legacyScreenSaver` (память растёт заметно медленнее; CPU в фоне ниже; заставка НЕ должна быть чёрной).

## Отложенное (нужна проверка на машине с Xcode, здесь нет)
- **Постоянная GPU-геометрия трейла** (SCNProgram/SCNBufferBinding + `.metal`), чтобы вообще не пересоздавать `SCNGeometry` каждый кадр. В этой песочнице `.metal`/SCNProgram не собрать и не увидеть — только на машине с Xcode.

## Инструментальная заметка
Serena без LSP не может править существующие Swift-файлы (`replace_content` → `No language servers available`). Для правки существующих рендер-файлов — нативный точечный `Edit`. См. `mem:development_guide`.
