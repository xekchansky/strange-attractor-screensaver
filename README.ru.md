# Strange Attractor Screensaver

Захватывающий скринсейвер, который симулирует и рендерит **странные аттракторы** —
хаотические динамические системы, рисующие бесконечно сложные, никогда не
повторяющиеся траектории в трёхмерном пространстве. Один движок на Rust — пять
способов смотреть: macOS, Windows, Linux, Android и браузер.

[![CI](https://github.com/xekchansky/strange-attractor-screensaver/actions/workflows/ci.yml/badge.svg)](https://github.com/xekchansky/strange-attractor-screensaver/actions/workflows/ci.yml)
[![Живое демо](https://img.shields.io/badge/%D0%B4%D0%B5%D0%BC%D0%BE-%D0%B2%20%D0%B1%D1%80%D0%B0%D1%83%D0%B7%D0%B5%D1%80%D0%B5-brightgreen)](https://xekchansky.github.io/strange-attractor-screensaver/)

![Анимация аттрактора Айзавы](docs/images/strange-attractor-aizawa-screensaver-animation.gif)

При каждом запуске случайно выбирается один из **12 классических хаотических
аттракторов**, случайным образом подбираются его параметры и камера, а
траектория рисуется светящейся градиентной лентой. В углу экрана показываются
живые формулы и текущие значения параметров — заставка, которая одновременно
является небольшим математическим арт-объектом.

→ English version: [README.md](README.md)

## Попробовать сразу

Без установки и сборки: **[откройте живое демо](https://xekchansky.github.io/strange-attractor-screensaver/)**
в браузере с WebGPU (или WebGL2). Выберите аттрактор и поделитесь URL — он
хранит точный seed.

## Установка

Все загрузки подписаны ad-hoc и **не нотаризованы**. Перед первым запуском
прочитайте примечание для своей платформы.

| Платформа | Загрузка | Перед первым запуском |
| --- | --- | --- |
| **macOS** 13+ | [`AttractorSaver.saver.zip`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Снять карантин: `xattr -r -d com.apple.quarantine ~/Downloads/AttractorSaver.saver` (см. [macOS](#macos)). |
| **Windows** 10+ | [`StrangeAttractor.scr.zip`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Распаковать, правый клик по `StrangeAttractor.scr` → **Установить**. SmartScreen может спросить — выберите **Подробнее → Выполнить в любом случае** (см. [Windows](#windows)). |
| **Linux** (x86_64) | [`strange-attractor-linux-x86_64.tar.gz`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Распаковать и прочитать `README.md` внутри; в архиве заставка, CLI для захвата и `.desktop`-файл (см. [Linux](#linux)). |
| **Android** 8+ | [`strange-attractor-android.apk`](https://github.com/xekchansky/strange-attractor-screensaver/releases) | Разрешить установку из браузера, затем выбрать **Strange Attractor** в *Настройки → Дисплей → Живые обои* (см. [Android](#android)). |
| **Браузер** | [Живое демо](https://xekchansky.github.io/strange-attractor-screensaver/) | Ничего устанавливать не нужно. |

Все архивы покрыты одним файлом `SHA256SUMS` в том же релизе:

```bash
sha256sum -c SHA256SUMS        # Linux
shasum -a 256 -c SHA256SUMS    # macOS
```

### macOS

1. Скачайте zip и файл контрольных сумм, распакуйте (Safari делает это сам) и
   при желании проверьте загрузку: `shasum -a 256 -c SHA256SUMS`.
2. Снимите карантин, чтобы Gatekeeper разрешил запуск ad-hoc подписанного
   бандла: `xattr -r -d com.apple.quarantine ~/Downloads/AttractorSaver.saver`.
   Если Homebrew-версия `xattr` перекрывает системную и падает с
   `bad interpreter`, вызывайте `/usr/bin/xattr` явно. Либо попробуйте открыть
   бандл один раз и разрешите его через **Системные настройки → Конфиденциальность
   и безопасность → Всё равно открыть**.
3. Двойной клик по `AttractorSaver.saver` — macOS откроет **Системные настройки →
   Обои → Экранная заставка**. Либо скопируйте бандл в
   `~/Library/Screen Savers/`.
4. Выберите **AttractorSaver** в списке заставок.

macOS 26 дополнительно требует подпись (ad-hoc достаточно) — неподписанный
`.saver` хост убивает с `Code Signature Invalid`. Нотаризация требует платного
аккаунта Apple Developer и сознательно отложена (решение D11).

### Windows

1. Скачайте и распакуйте `StrangeAttractor.scr.zip`.
2. Правый клик по `StrangeAttractor.scr` → **Установить**, либо скопируйте файл
   вручную в `C:\Windows\System32` и выберите **Strange Attractor** в
   *Параметры → Персонализация → Экран блокировки → Параметры экранной заставки*.
3. Бинарник не подписан, поэтому SmartScreen покажет предупреждение для свежей
   загрузки; выберите **Подробнее → Выполнить в любом случае**.

`.scr` поддерживает протокол экранных заставок Windows: `/s` запускает,
`/p HWND` рисует в превью панели управления, `/c` открывает небольшое окно
настроек (выбор аттрактора хранится в
`%APPDATA%\StrangeAttractor\settings.json`), пустая командная строка открывает
настройки.

### Linux

В архиве лежат `attractor-linux-saver` (одновременно hack для xscreensaver и
полноэкранный runner), `attractor-cli` (захват и демо),
`strange-attractor.desktop` и Linux-`README.md` с полным рецептом. Коротко:

```bash
install -Dm755 attractor-linux-saver ~/.local/bin/attractor-linux-saver
install -Dm644 strange-attractor.desktop ~/.local/share/applications/strange-attractor.desktop
attractor-linux-saver --fullscreen      # Wayland или любой лаунчер
```

Для xscreensaver добавьте `"Strange Attractor" attractor-linux-saver -root` в
`~/.xscreensaver` и перезапустите демон. Файл настроек следует XDG:
`${XDG_CONFIG_HOME:-~/.config}/strange-attractor/settings.json`. Архив собран
против glibc CI-раннера; на более старом дистрибутиве собирайте из исходников.

### Android

1. Скачайте `strange-attractor-android.apk` и откройте; разрешите установку из
   этого источника, когда Android спросит.
2. Откройте **Настройки → Дисплей → Живые обои → Strange Attractor**. Пикер в
   настройках обоев выбирает аттрактор (или *Случайный каждый раз*).

Опубликованный APK подписан **debug-ключом**, потому что для release-подписи
нужен keystore, которого не может быть в репозитории. Он устанавливается и
работает, но публикация в магазине не настроена. Обои рендерятся через GLES,
по одному плееру на каждую видимую поверхность.

## Возможности

- **12 странных аттракторов** — Лоренц, Айзава, Рёсслер, Томас, Чен, Лу, Чуа,
  Халворсен, Мур–Шпигель и семейство Спротта (см. [каталог](#каталог-аттракторов)).
- **Один движок, пять оболочек** — один и тот же Rust-core и рендерер на `wgpu`
  рисуют macOS `.saver`, Windows `.scr`, Linux-hack, Android-обои и браузерное
  демо.
- **Данные вместо кода** — аттракторы описаны в едином JSON-каталоге (формулы,
  диапазоны параметров, начальные состояния), а не в коде. Новый аттрактор
  добавляется без единой строчки кода.
- **Интегрирование Рунге–Кутты 4-го порядка**: число шагов на кадр ограничено,
  поэтому подвисание системы после сна не замораживает анимацию.
- **Самолечение** — сторожевой механизм распознаёт вырожденные запуски
  (расходимость, схлопывание в точку, устойчивые периодические орбиты,
  нефинитные состояния) и бесшовно перезапускает симуляцию с новым случайным
  набором.
- **4 цветовых режима трейла** — радуга вдоль траектории, монохром по
  устареванию, глубина от камеры, окраска по координатам.
- **Парсинг формул в рантайме** — уравнения `dx/dy/dz` разбираются из строк,
  поэтому каталог остаётся чистыми данными.
- **Воспроизводимый захват** — `attractor-cli` точно повторяет любой кадр
  (`--attractor`, `--seed`, `--screenshot`); так сделаны картинки ниже и
  регрессионные фикстуры.

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

Параметры и гиперпараметры симуляции (шаг интегрирования, длина трейла,
скорость камеры, скорость смены цвета) выбираются случайно из отобранных
диапазонов в [`data/attractors.json`](data/attractors.json). Каталог проверяется
при загрузке — формулы должны разбираться, диапазоны быть корректными, а
функции — поддерживаемыми; юнит-тесты интегрируют каждую запись каталога.
Хаотические семейства с крошечными бассейнами притяжения (например, Sprott D–J,
Рабиновича–Фабриканта) исключены — от случайных стартов они расходятся.
Картинки выше переснимаются скриптом
[`docs/capture_all.sh`](docs/capture_all.sh) через Rust-CLI захвата, с
фиксированным seed и закреплёнными ракурсами.

## Как это устроено

```text
data/attractors.json ──▶ attractor-core ──▶ attractor-view ──▶ attractor-gpu
   каталог + формулы      RK4-движок,        камера, геометрия    лента wgpu +
                          health monitor     трейла, оверлей     текст оверлея
                                                  │
                                                  ▼
                                            attractor-player
                                     tick / render / resize / restart
                                                  │
        ┌──────────────┬──────────────┬────────────┴──────────┬──────────────┐
        ▼              ▼              ▼                       ▼              ▼
   macOS .saver   Windows .scr   Linux hack / runner    Android JNI     wasm demo
     (FFI +         (Win32)          (X11/winit)          (Wallpaper       (canvas
    AppKit view)                                          Service)        в браузере)
```

- **`rust/crates/attractor-core`** — чистый движок без UI: модель данных,
  рекурсивный парсер формул, RK4-решатель, движок с кольцевым буфером трейла,
  рандомайзер и health monitor.
- **`rust/crates/attractor-view`** — слой представления без GPU: орбитальная
  камера с автокадрированием, геометрия градиентной ленты, все четыре цветовых
  режима, модель оверлея.
- **`rust/crates/attractor-gpu`** — конвейеры `wgpu`: лента с мультисэмплингом,
  текстовый оверлей, чтение кадра в PNG и платформенные surface-таргеты.
- **`rust/crates/attractor-player`** — рантайм, который встраивает каждая
  оболочка.
- **`rust/crates/attractor-ffi`** — C ABI для macOS-хоста и Android JNI;
  **`attractor-wasm`** — браузерная обвязка.
- **`rust/crates/attractor-windows-saver`** и
  **`rust/crates/attractor-linux-saver`** — нативные оболочки.
- **`rust/shells/android`** — Kotlin `WallpaperService` и пикер.
- **`Host/`** — macOS `.saver`, тонкое Swift/AppKit-представление над C ABI.

Плеер перевыбирает аттрактор, когда watchdog видит плохой запуск (траектория
схлопнулась в точку или ушла в периодическую петлю), поэтому заставка никогда
не замирает на сломанном кадре.

## Сборка из исходников

Для движка достаточно стабильного Rust (edition 2024); оболочкам нужны Xcode,
Windows SDK или Android SDK/NDK.

```bash
git clone https://github.com/xekchansky/strange-attractor-screensaver.git
cd strange-attractor-screensaver

# Тесты движка и проверка workspace
cd rust
cargo test --workspace

# Захват кадра (нужен реальный GPU, поэтому запускайте в GUI-сессии)
cargo run --release -p attractor-cli -- \
  --attractor lorenz --seed 42 --screenshot lorenz.png
```

CLI захвата повторяет флаги старого демо-приложения: `--attractor`,
`-p name=value` (можно повторять), `--step`, `--speed`, `--cam-speed`,
`--trail-color-speed`, `--trail-length`, `--color-mode`, `--screenshot`,
`--record`, `--record-seconds`, `--frame-stride`, `--capture-scale`,
`--warmup-steps`, `--cam-azimuth`, `--cam-elevation`, `--initial-state`, плюс
`--seed`, `--frames`, `--width` и `--height`.

### macOS-заставка

```bash
cd Host && xcodegen generate && cd ..
./scripts/package_release.sh            # пишет dist/AttractorSaver.saver.zip
```

`ARCHS="arm64 x86_64"` собирает универсальный бандл (нужны обе стандартные
библиотеки Rust: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`).

### Windows-заставка

```powershell
cargo build --release -p attractor-windows-saver
Copy-Item rust\target\release\attractor-windows-saver.exe StrangeAttractor.scr
```

### Linux

```bash
cd rust
cargo build --release -p attractor-linux-saver -p attractor-cli
./scripts/package_linux.sh              # из корня репозитория
```

Подробности про xscreensaver, `swayidle`/`hypridle` и настройки —
в [`rust/shells/linux/README.md`](rust/shells/linux/README.md).

### Android

```bash
cd rust/shells/android
rustup target add aarch64-linux-android x86_64-linux-android
cargo install --locked cargo-ndk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018
./build-native.sh                       # jniLibs для APK
./gradlew assembleDebug
```

Ручной чек-лист для эмулятора и телефона —
в [`rust/shells/android/README.md`](rust/shells/android/README.md).

### Браузерное демо

```bash
bash web/build.sh
cd web && npm ci && npm test
```

## Требования и проверенное окружение

- **Движок:** стабильный Rust (edition 2024), GPU для тестов не нужен —
  тесты конвейеров используют бэкенд `noop` из `wgpu`. Захвату и оболочкам
  нужен реальный GPU-адаптер.
- **macOS:** 13 или новее, Apple Silicon или Intel; Xcode и
  [xcodegen](https://github.com/yonaskolb/XcodeGen) для бандла. Проверено на
  macOS 26 со стабильным Rust и Xcode 26.
- **Windows:** 10 или новее с DirectX 12 (или Vulkan). CI собирает и рендерит
  через WARP на `windows-latest`.
- **Linux:** X11 или Wayland с Vulkan или GLES; CI прогоняет X11-пути под Xvfb
  с программным Vulkan-драйвером Mesa.
- **Android:** 8.0+ (API 26), `arm64-v8a` и `x86_64`; NDK 27.2.
- **Браузер:** WebGPU с автоматическим откатом на WebGL2.

## Решение проблем

- **macOS-заставка не появляется в Системных настройках.** Перекопируйте бандл
  через `ditto` (не `cp -R`) и перезапустите агент обоев:
  `killall WallpaperAgent`. Система кеширует список заставок.
- **macOS-заставка показывает «Invalid Page» или чёрное превью.** Обычно хост
  всё ещё держит старую копию. Закройте Системные настройки, удалите бандл,
  скопируйте заново и откройте панель снова.
- **Загрузка для macOS блокируется Gatekeeper.** См. шаг с карантином
  в разделе [macOS](#macos).
- **Windows показывает «Windows защитила ваш компьютер».** `.scr` не подписан;
  выберите **Подробнее → Выполнить в любом случае** или купите сертификат
  подписи кода и подпишите бинарник сами.
- **Оболочка запускается и сразу выходит.** Нужен рабочий GPU-адаптер; сначала
  прогоните `attractor-cli --screenshot` — ошибка назовёт точную причину
  (`no GPU adapter`, ошибка поверхности, ошибка каталога).

## Заметки и известные проблемы

- Установка **неподписанной** `.saver` на macOS 26 может заблокировать глобальные
  горячие клавиши сессии (Cmd+Tab, переключение раскладки) до перезагрузки — см.
  [разбор](.serena/memories/macos_input_lockup.md). Релизные бандлы подписаны
  ad-hoc, система их принимает.
- Опубликованный Android APK несёт debug-подпись (см. [Android](#android));
  macOS-бандлы не нотаризованы (D11); Windows `.scr` не подписан.

## Документация

- [README на английском](README.md)
- Заметки для контрибьюторов и кодирующих агентов: [`AGENTS.md`](AGENTS.md)
- Статус итераций, решения и отложенная работа:
  [`rust/STATUS.md`](rust/STATUS.md)
- Кросс-платформенный дизайн и ADR:
  [`docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md`](docs/superpowers/specs/2026-11-12-rust-cross-platform-design.md)
- Скрипт захвата ассетов документации: [`docs/capture_all.sh`](docs/capture_all.sh)
- Упаковка релиза: [`scripts/package_release.sh`](scripts/package_release.sh),
  [`scripts/package_linux.sh`](scripts/package_linux.sh)
- Политика безопасности: [`SECURITY.md`](SECURITY.md)

## Лицензия

[MIT](LICENSE) © Khakayu Alexandr
