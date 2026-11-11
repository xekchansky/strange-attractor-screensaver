# Strange Attractor Screensaver

Захватывающий скринсейвер для macOS, который симулирует и рендерит **странные аттракторы** — хаотические динамические системы, рисующие бесконечно сложные, никогда не повторяющиеся траектории в трёхмерном пространстве.

![Анимация аттрактора Айзавы](docs/images/strange-attractor-aizawa-screensaver-animation.gif)

При каждом запуске случайно выбирается один из **12 классических хаотических аттракторов**, случайным образом подбираются его параметры и камера, а траектория рисуется светящейся градиентной линией. В углу экрана показываются живые формулы и текущие значения параметров — заставка, которая одновременно является небольшим математическим арт-объектом.

→ English version: [README.md](README.md)

## Возможности

- **12 странных аттракторов** — Лоренц, Айзава, Рёсслер, Томас, Чен, Лу, Чуа, Халворсен, Мур–Шпигель и семейство Спротта (см. [каталог](#каталог-аттракторов)).
- **Данные вместо кода** — аттракторы описаны в едином JSON-каталоге (формулы, диапазоны параметров, начальные состояния), а не в коде. Новый аттрактор добавляется без единой строчки кода.
- **Интегрирование Рунге–Кутты 4-го порядка**: число шагов на кадр ограничено, поэтому подвисание системы после сна не замораживает анимацию.
- **Самолечение** — сторожевой механизм распознаёт вырожденные запуски (расходимость, схлопывание в точку, устойчивые периодические орбиты, нефинитные состояния) и бесшовно перезапускает симуляцию с новым случайным набором.
- **4 цветовых режима трейла** — радуга вдоль траектории, монохром по устареванию, глубина от камеры, окраска по координатам.
- **Парсинг формул в рантайме** — уравнения dX/dY/dZ разбираются из строк, поэтому каталог остаётся чистыми данными.
- **Без сторонних зависимостей в рантайме** — только Swift и фреймворки Apple (Foundation, SceneKit, AppKit). Тесты используют только Swift Testing.
- **Демо-приложение с CLI** — запуск любого аттрактора с явными параметрами для исследования и отладки.

## Каталог аттракторов

| Аттрактор | Уравнения | Параметры | Превью |
|---|---|---|---|
| **Лоренц** | `dx = a(y − x)`<br>`dy = x(r − z) − y`<br>`dz = xy − bz` | a, b, r | ![Лоренц](docs/images/attractor-lorenz-simulation.png) |
| **Айзава** | `dx = (z − b)x − dy`<br>`dy = dx + (z − b)y`<br>`dz = c + az − z³/3 − (x² + y²)(1 + ez) + fz x³` | a, b, c, d, e, f | ![Айзава](docs/images/attractor-aizawa-simulation.png) |
| **Рёсслер** | `dx = −y − z`<br>`dy = x + ay`<br>`dz = b + z(x − c)` | a, b, c | ![Рёсслер](docs/images/attractor-rossler-simulation.png) |
| **Томас** | `dx = sin(y) − bx`<br>`dy = sin(z) − by`<br>`dz = sin(x) − bz` | b | ![Томас](docs/images/attractor-thomas-simulation.png) |
| **Чен** | `dx = a(y − x)`<br>`dy = (c − a)x − xz + cy`<br>`dz = xy − bz` | a, b, c | ![Чен](docs/images/attractor-chen-simulation.png) |
| **Лу** | `dx = a(y − x)`<br>`dy = −xz + cy`<br>`dz = xy − bz` | a, b, c | ![Лу](docs/images/attractor-lu-simulation.png) |
| **Чуа** (двойной скролл) | `dx = α(y − x − f(x))`<br>`dy = x − y + z`<br>`dz = −βy` с кусочно-линейной `f(x)` | α, β, m₀, m₁ | ![Чуа](docs/images/attractor-chua-simulation.png) |
| **Халворсен** (циклически-симметричный) | `dx = −ax − 4y − 4z − y²`<br>`dy = −ay − 4z − 4x − z²`<br>`dz = −az − 4x − 4y − x²` | a | ![Халворсен](docs/images/attractor-halvorsen-simulation.png) |
| **Мур–Шпигель** (осциллятор 2-го порядка) | `dx = 10y`<br>`dy = 35z`<br>`dz = −z − (T − R + Rx²)y/35 − (T/350)x` | R, T | ![Мур–Шпигель](docs/images/attractor-moore_spiegel-simulation.png) |
| **Спротт A** | `dx = y`<br>`dy = −x + yz`<br>`dz = 1 − y²` | фикс. | ![Спротт A](docs/images/attractor-sprott_a-simulation.png) |
| **Спротт B** | `dx = yz`<br>`dy = x − y`<br>`dz = 1 − xy` | фикс. | ![Спротт B](docs/images/attractor-sprott_b-simulation.png) |
| **Спротт C** | `dx = yz`<br>`dy = x − y`<br>`dz = 1 − x²` | фикс. | ![Спротт C](docs/images/attractor-sprott_c-simulation.png) |

Параметры и гиперпараметры симуляции (шаг интегрирования, длина трейла, скорость камеры, скорость смены цвета) выбираются случайно из отобранных диапазонов в [`Core/Sources/AttractorCore/Resources/attractors.json`](Core/Sources/AttractorCore/Resources/attractors.json). Каталог проверяется при загрузке — формулы должны разбираться, диапазоны быть корректными, а функции — поддерживаемыми; юнит-тесты интегрируют каждую запись каталога. Хаотические семейства с крошечными бассейнами притяжения (например, Sprott D–J, Рабиновича–Фабриканта) исключены — от случайных стартов они расходятся.

## Установка

### Из релиза (рекомендуется)

1. Скачайте `AttractorSaver.saver.zip` (и файл `.sha256`) из последнего [релиза](https://github.com/xekchansky/strange-attractor-screensaver/releases) и распакуйте (Safari делает это сам). При желании проверьте загрузку:

   ```bash
   shasum -a 256 -c AttractorSaver.saver.zip.sha256
   ```

2. Бандл подписан ad-hoc и **не нотаризован**, поэтому Gatekeeper заблокирует скачанную копию с сообщением «Apple не удалось подтвердить…». Сначала снимите карантинный флаг:

   ```bash
   xattr -r -d com.apple.quarantine ~/Downloads/AttractorSaver.saver
   ```

   Если в PATH стоит Homebrew-версия `xattr` и падает с `bad interpreter`, вызывайте `/usr/bin/xattr`. Альтернатива — один раз попробовать открыть бандл и разрешить его через **Настройки системы → Конфиденциальность и безопасность → Всё равно открыть**.

3. Откройте `AttractorSaver.saver` двойным кликом — macOS предложит установить его в **Настройки системы → Обои → Заставка**. Либо скопируйте его в `~/Library/Screen Savers/`.
4. Выберите **AttractorSaver** в списке заставок.

На macOS 26 бандл дополнительно обязан быть подписан (достаточно ad-hoc) — неподписанный `.saver` хост убивает с `Code Signature Invalid`.

### Сборка из исходников

Понадобятся macOS с Xcode (или Xcode Command Line Tools) и [xcodegen](https://github.com/yonaskolb/XcodeGen):

```bash
git clone https://github.com/xekchansky/strange-attractor-screensaver.git
cd strange-attractor-screensaver

# Юнит-тесты ядра (Xcode не требуется)
cd Core && swift test && cd ..

# Сборка и установка .saver-бандла
cd Host
xcodegen generate
xcodebuild -project AttractorHost.xcodeproj -scheme AttractorSaver \
  -configuration Release -derivedDataPath /tmp/host-dd build
mkdir -p ~/Library/Screen\ Savers
rm -rf ~/Library/Screen\ Savers/AttractorSaver.saver
ditto /tmp/host-dd/Build/Products/Release/AttractorSaver.saver \
      ~/Library/Screen\ Savers/AttractorSaver.saver
```

Чтобы собрать zip для релиза и контрольную сумму, запустите
[`scripts/package_release.sh`](scripts/package_release.sh).

### Попробовать без установки: демо-приложение

Автономное демо позволяет запустить конкретный аттрактор с явными параметрами — удобно для исследования пространства параметров:

```bash
cd Demo && swift run AttractorDemo --attractor aizawa
```

Демо требует GUI-сессию: оно открывает настоящее окно, и живой вид, и съёмка PNG используют Metal.

Опции командной строки:

```text
-a, --attractor <id>       выбрать конкретный аттрактор (список — в --help)
-p, --param <имя>=<значение>  переопределить параметр (повторяемо, требует --attractor)
--step <v>                 шаг интегрирования
--speed <v>                множитель скорости симуляции
--cam-speed <v>            скорость вращения камеры
--trail-color-speed <v>    скорость смены цвета трейла
--trail-length <n>         длина трейла в точках
--color-mode <mode>        trail | age | depth | spatial
--screenshot <path>        снять один PNG прогретой сцены и выйти
--record <dir>             записать PNG-кадры (для сборки гифки) и выйти
--record-seconds <v>       длительность записи (по умолчанию 4)
--frame-stride <n>         снимать каждый n-й кадр (по умолчанию 2)
--capture-scale <n>        множитель разрешения снимка (по умолчанию 2)
--warmup-steps <n>         шагов интегрирования перед съёмкой (по умолчанию 60000)
--cam-azimuth <deg>        начальный азимут орбиты камеры (по умолчанию случайный)
--cam-elevation <deg>      высота камеры над горизонтальной плоскостью (по умолчанию 36)
--initial-state x,y,z      начальная точка интегрирования
```

## Как это устроено

```
attractors.json ──▶ AttractorCatalog ──▶ Randomizer ──▶ AttractorSelection
                                                              │
                                                              ▼
   SceneKit view ◀── TraceRenderer ◀── AttractorEngine ◀── AttractorModel
   (камера, трейл)                      ▲              (парсер формул
                                        │                рекурсивным спуском)
                                  ODESolver (RK4)
```

- **`Core/`** — `AttractorCore`: чистый SPM-модуль без UI. Модель данных, парсер формул (рекурсивный спуск — `NSExpression` не умеет `sin`/`abs`), решатель RK4, движок с кольцевым буфером трейла, рандомизатор и сторожевой монитор, проверяющий каждое интегрированное состояние.
- **`Render/`** — `AttractorRender`: рендер на SceneKit — градиентная полилиния трейла, орбитальная камера с автокадрированием, оверлей формул. Общий для демо и заставки.
- **`Demo/`** — `AttractorDemo`: автономное AppKit-приложение для интерактивных запусков и offscreen-съёмки кадров (`--screenshot` / `--record`).
- **`Host/`** — таргеты заставки: классический `.saver`, который устанавливают пользователи, и экспериментальный вариант `.appex` для встраивания в приложение-контейнер. Оба генерируются xcodegen из `Host/project.yml`.

Заставка пересоздаёт всю сцену, когда сторож обнаруживает неудачный запуск (например, траектория схлопнулась в точку или застряла в периодической петле), поэтому она никогда не замирает на сломанном кадре.

## Если что-то пошло не так

- **Заставки нет в Настройках системы.** Скопируйте бандл через `ditto` (не
  `cp -R`) и перезапустите агент обоев: `killall WallpaperAgent`. Система
  кеширует список заставок.
- **Заставка показывает «Invalid Page» или чёрное превью.** Обычно хост всё ещё
  держит старую копию. Закройте Настройки системы, удалите бандл, скопируйте
  заново и откройте панель снова.
- **Gatekeeper блокирует загрузку.** См. шаг с карантином в разделе
  [Установка](#установка).

## Требования и проверенная конфигурация

- macOS 13 или новее, Apple Silicon или Intel.
- Проверено на macOS 26 (Apple Silicon): Xcode 26, Swift 6.3.
- Для сборки заставки нужны Xcode и [xcodegen](https://github.com/yonaskolb/XcodeGen);
  тесты ядра работают и с Command Line Tools.

## Примечания и известные проблемы

- Установка **неподписанного** `.saver` на macOS 26 может подвесить глобальные
  горячие клавиши сессии (Cmd+Tab, переключение раскладки) до перезагрузки — см.
  [описание](.serena/memories/macos_input_lockup.md). Релизные бандлы подписаны
  ad-hoc, и система их принимает.

## Документация

- [README на английском](README.md)
- Заметки для контрибьюторов и агентов: [`AGENTS.md`](AGENTS.md)
- Политика безопасности: [`SECURITY.md`](SECURITY.md)
- Скрипт съёмки документационных ассетов: [`docs/capture_all.sh`](docs/capture_all.sh)
- Сборка релиза: [`scripts/package_release.sh`](scripts/package_release.sh)

## Лицензия

[MIT](LICENSE) © Khakayu Alexandr
