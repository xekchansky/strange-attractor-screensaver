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
- `SimulationScene`: (а) per-frame `autoreleasepool` в рендер-шаге; (б) полилиния пересобирается **каждые 2 кадра** (`renderFrame % 2 == 0`, ~30 Гц) — пофреймовая пересборка `SCNGeometry` давала рваную отрисовку превью, троттлинг по числу точек — дёрганье кончика; (в) **ручной run-loop `Timer` удалён полностью** — он был первопричиной роста памяти (см. итерация 5).
- **Код-подпись `.saver`:** unsigned-бандл (был `CODE_SIGNING_ALLOWED: NO`, linker-signed) **крашит превью** настроек обоев: `WallpaperLegacyExtension` падает `SIGKILL (Code Signature Invalid)`/`Invalid Page` на `dlopen_preflight`. В `Host/project.yml` для `AttractorSaver` включено `CODE_SIGN_IDENTITY: "-"` (ad-hoc), чтобы сборка давала валидную подпись. Ручной фикс: `codesign --force --sign - <saver>`.
- Вывод/цвета не изменены. Ветка `feature/m6-perf-mitigate` — проверить в Activity Monitor за `legacyScreenSaver` (память растёт заметно медленнее; CPU в фоне ниже; заставка НЕ должна быть чёрной).

## Изменение (итерация 5): слабый run-loop таймер вместо сильного target
**Первопричина** роста памяти нашлась в `Timer`/`NSObject`: классический `Timer(timeInterval:target:self, selector:...)` даёт retain-цикл `RunLoop.main → Timer → сцена` (target сильный) → сцена с большим `SCNView` не деаллоцируется даже когда превью-хост отпускает `ScreensaverView` (а он НЕ зовёт `stopAnimation()`). Каждый запуск превью добавлял ещё одну вечно живую сцену → `legacyScreenSaver` рос 229→500+ МБ за цикл, до 2.6 ГБ.

Сначала пробовали убрать таймер в пользу встроенного display link `SCNView` (`SCNSceneRendererDelegate`+`isPlaying`, conformance `@preconcurrency`) — **получился чёрный экран**: в хост-заставке (`WallpaperLegacyExtension`) display link view не стартует, делегат не вызывается, геометрия не строится. (Бонус: `scnView.delegate = self` нельзя до `super.init()`; ObjC-протокол не main-actor → `@preconcurrency` обязателен иначе `conformance-isolation` — ошибка сборки; из `deinit` нельзя трогать main-actor свойства — только через `MainActor.assumeIsolated`.) Этот вариант откачен.

Решение (в `SimulationScene` сейчас): **run-loop `Timer` с `[weak self]`-блоком**:
- `Timer(timeInterval: 1/60, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.tick() } }` + `RunLoop.main.add(... forMode: .common)`. Блок держит weak self → сцена освобождается; `tick()` строит геометрию, а SceneKit перерисовывает кадр при замене `traceNode.geometry` (рабочий механизм в хосте).
- `stop()`/`deinit` (через `MainActor.assumeIsolated`) — `timer?.invalidate()`; `deinit` ещё `view.scene = nil`.
- Guard-ы видимости окна НЕ используем (`isVisible`/`window == nil`/`occlusionState` на macOS 26 ненадёжны и давали чёрный экран).
- Пересборка полилинии каждые 2 кадра ~30 Гц, per-frame `autoreleasepool`, persistent `SCNMaterial` — остались.
- Визуально: заставка/превью двигаются.

## Статус 2026-10-01: рост памяти при перезапусках НЕ решён (отложено)
Пользователь: «Проблема осталась, после перезапусков память растет, запомни промежуточные результаты, вернёмся позже». Т.е. слабый таймер восстановил показ, но **память `legacyScreenSaver` продолжает расти с каждым запуском превью** → сильный `target:` retain-цикл был НЕ единственной (или не главной) причиной удержания старых сцен.

### Измерения пользователя (2026-09-30)
- `legacyScreenSaver`: ~229 МБ → 513 МБ после одного открытия/закрытия превью; до ~2.6 ГБ при долгой эксплуатации.
- Процесс превью (`WallpaperLegacyExtension`/обои): 710 → 806 → 900 → 989 МБ при последовательных запусках.

### Что отмечено и исключено
- Исключено: сильный `target:self` таймера (слабел, роста нет), guard-ы видимости окна (`isVisible`/`window == nil`/`occlusion`) — ненадёжны на macOS 26, давали чёрный экран; display link `SCNView` не стартует в хосте (чёрный экран).
- Движок корректен: `AttractorEngine.trace` — кольцевой буфер фиксированной ёмкости, вне буфера не растёт.
- Действующие меры (оставить): persistent `SCNMaterial`, per-frame `autoreleasepool`, пересборка полилинии каждые 2 кадра ~30 Гц, weak-таймер + `deinit` invalidate.

### Гипотезы для следующего подхода
1. Сцены/`ScreensaverView` удерживает сам хост `WallpaperLegacyExtension` (а не наш таймер) → ограничить footprint одной сцены или переиспользовать сцену между запусками.
2. Рост может идти ВНУТРИ одного длинного показа (не только между перезапусками) — проверить тренд во время непрерывного показа отдельно.
3. SceneKit удерживает старые `SCNGeometry`/GPU-ресурсы дольше смены `node.geometry` → постоянная GPU-геометрия (SCNProgram+MTLBuffer) остаётся главным кандидатом.
4. Debug через Instruments/Allocations — только на машине с Xcode (здесь SPM-сборка без UI).

## Результат 2026-10-01 (вечер): демо-замер — рендер НЕ течёт
В `Demo/Sources/AttractorDemo/AppDelegate.swift` добавлен временный RSS-логгер (`[rss] t=Ns  MB`, `task_info`/`phys_footprint`, каждые 5 с). Прогон одного показа ~6 минут в обычном AppKit-окне: RSS стабилен 164–180 МБ, **линейного роста нет**. Единственное число 167/164 — нормальные колебания GC/footprint.

**Вывод:** per-frame пересоздание `SCNGeometry`+`Data`/`SCNGeometrySource` **НЕ копит** память внутри одного показа (в демо-конфигурации). Кандидат №3 из «Гипотез» (постоянная GPU-геометрия SCNProgram/MTLBuffer для роста памяти) — **снят/не нужен**; не заниматься им, пока не будет данных из хоста об обратном.

**Значит источник роста — хост-путь** (`legacyScreenSaver` при показах заставки), а не наш per-frame рендер. Наблюдение «229→513 МБ за один показ» + «рostный процесс не растёт в демо» согласуются с тем, что каждый показ в хосте оставляет неосвобождённым объект уровня «сцена+вью» (~280 МБ).

### Сегодняшние правки (ветка feature/m6-perf-mitigate, не закоммиченные)
- `Host/Sources/ScreensaverView.swift`: единый `tearDownScene()` = `stop()` + **`view.removeFromSuperview()`** + обнуление ссылки; вызывается из `stopAnimation()` и перед созданием новой сцены. Причина: превью-хост может переиспользовать один `ScreensaverView`, оставляя старые SCNView (всю сцену) в subviews — «+одна сцена за показ».
- Логирование — `os.Logger`: subsystem `io.github.xekchansky.AttractorSaver` (category `lifecycle`) в `ScreensaverView` (проблемы каталога, сбой старта, отложенный перезапуск), subsystem `io.github.xekchansky.AttractorRender` (category `scene`) в `SimulationScene` (запрос перезапуска). Для диагностики: `log stream --level debug --predicate 'subsystem == "io.github.xekchansky.AttractorSaver"'`.

### Проблема: измерять хост пока не удаётся (системная нестабильность)
- Предпросмотр в System Settings: **крах** — «Extension process `Обои` (WallpaperLegacyExtension) exited» (могло быть эпизодическим; на сборке 17:47 превью «бежал» ~10 с на той же установке).
- `open -a /System/Library/CoreServices/ScreenSaverEngine.app` → **чёрный экран**; в логе видно, что путь идёт через WallpaperAgent (`makeWallpaper`, `Initialize screen saver wallpaper`) — на macOS 26 заставка активируется через Wallpaper-движок, вызова нашего `startAnimation` в стриме не видно.
- AMFI: `amfid ... not valid ... adhoc signed` (Code -423) — не блокирует запуск, но звучит при загрузке .saver.

### Что проверить дальше
1. Baseline: откатить рабочие изменения (`git stash`) → собрать/поставить → убедиться, что превью возвращается к жизни. Если и baseline крашится — системная нестабильность/подпись/новый API (рассмотреть `ScreenSaverKit`/`SAView` macOS 26), а не правки.
2. Если превью жив: снять `log stream` по subsystem → увидеть, вызывается ли `startAnimation`, и приходят ли `dealloc` после закрытия (главный вопрос: держит ли хост сцену).
3. Возможный правый ветер: рост между показами при том, что хост фабрикует новые `ScreensaverView` (новые адреса view в логе) — старые могут не освобождаться, либо это Wallpaper-engines копит своё.

## ✅ РЕШЕНИЕ (2026-10-01, вечер): корневая причина найдена и исправлена
**Рост памяти в хосте решён.** Память `legacyScreenSaver` при повторных открытиях превью держится ~200 МБ (было: +~130 МБ/показ, 5 показов → 700 МБ). Деаллокация сцен подтверждена в логе.

### Первопричина (подтверждена live-счётчиком)
`legacyScreenSaver` при каждом показе заставки создаёт **новый** `ScreensaverView` (новые адреса view, см. `setScreenSaverModule`, activity-ids в логе), вызывает `startAnimation` и **никогда**:
- не вызывает `stopAnimation()`,
- не освобождает предыдущие `ScreensaverView` (держивает их в себе навсегда).

Каждый удерживаемый `ScreensaverView` держал свою тяжёлую `SimulationScene` (SCNView+SCNScene+геометрия) → **+1 живая сцена за показ**. Live-счётчик это показал прямо: `#0 (live=1) → #1 (live=2) → … → #5 (live=6)`, ноль `dealloc`. Слабый таймер и `removeFromSuperview` на stop-пути не спасали: следующий `startAnimation` происходит на **новом** view, а старый view без teardown остаётся со сценой.

### Фикс (в `Host/Sources/ScreensaverView.swift`)
Хост сигнализирует об окончании показа единственным надёжным событием — **детaчем view из иерархии**: `viewDidMoveToSuperview` с `superview == nil` (приходит парой с `window = false`). Реализовано:
```swift
override func viewDidMoveToSuperview() {
    super.viewDidMoveToSuperview()
    if self.superview == nil { tearDownScene() }
}
```
`tearDownScene()` = `scene?.stop()` + `scene?.view.removeFromSuperview()` + `scene = nil` → `SimulationScene` деаллоцируется (`deinit` = invalidate таймера + `view.scene = nil`). Удерживаемый хостом `ScreensaverView` остаётся лёгким (без сцены); следующий `startAnimation` строит свежую сцену.

Сопутствующее — временная диагностика **вычищена 2026-10-01** (ос.Logger-точки, `sceneID`/`live`-счётчик в `SimulationScene`, RSS-логгер в демо). Методика замера сохранена в этой памяти. В коде остался только сам фикс (`viewDidMoveToSuperview` → `tearDownScene`).

### Открытые вопросы к фиксу (проверка реальной заставкой)
1. ✅ **Р ЕАЛЬНЫЙ показ проверен (2026-10-01, вечер):** запуск по таймауту, переключение приложений/пространств, ввод пароля — анимация продолжается, сцена не сбрасывается, память не растёт. Grace-период НЕ понадобился: при активном показе view из иерархии не детачится (`superview == nil` приходит только при завершении показа).
2. Сценарии многомониторности (view на разных экранах) — не тестировались.

### Методика диагностики (пригодится впредь)
- `log stream --level debug --predicate 'subsystem == "io.github.xekchansky.AttractorSaver"'`.
- Live-счётчик сцен (created/dealloc) — прямо показывает накопление.
- RSS-замер в демо отделил «наш рендер не течёт» (стабильные 164–180 МБ за 6 мин) от «течёт хост».

## Отложенное (нужна проверка на машине с Xcode, здесь нет)
- **Постоянная GPU-геометрия трейла** (SCNProgram/SCNBufferBinding + `.metal`) — была кандидатом на рост памяти, но демо-замер 2026-10-01 показал, что per-frame геометрия не течёт. Отложено до данных из хоста; если хост-рост окажется из GPU-буферов — вернуться.

## Инструментальная заметка
Serena без LSP не может править существующие Swift-файлы (`replace_content` → `No language servers available`). Для правки существующих рендер-файлов — нативный точечный `Edit`. См. `mem:development_guide`.
