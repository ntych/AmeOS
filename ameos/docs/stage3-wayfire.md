# Wayfire и Wayland сессия для AmeDE (этап 3)

## Решения

**Compositor:** Wayfire 0.8.1+ (официальный FreeBSD порт `wayland/wayfire`).
Как выбрали: Wayfire стабилен на FreeBSD, modular (plugins), и документирован
в [FreeBSD Handbook](https://docs.freebsd.org/en/books/handbook/wayland/).

**Сессия:** systemd-логин *не* на FreeBSD. Используем:
- `/usr/local/bin/amede-session` — shell-скрипт, запускает Wayfire и AmeDE
- XDG session type: `wayland` (хотя Wayfire и не GNOME/KDE, следует стандарту)
- `/usr/local/share/wayland-sessions/amede.desktop` — регистрация сессии

**Пакеты (для desktop-amd64):**
- wayland (base lib)
- libxkbcommon (keyboard input)
- libpixman (rasterization)
- wayfire (compositor)
- wf-shell (панель + dock за нас писать не будут, Wayfire используется без неё)
- mesa-libs (OpenGL)
- xwayland (X11 совместимость для `xterm`, `foot`)
- foot (Wayland-native terminal) или xterm (через XWayland)
- liberation-fonts или dejavu-fonts (для текста)

**Wayfire конфиг:** `/etc/xdg/wayfire/wayfire.ini` + overlay-инстолляция в
`overlay/etc/xdg/wayfire/` с подстановкой токенов (пока нет).

**Гарантии:**
- На других WM/DE сессия не меняется; пользователь всё ещё может выбрать
  другую в display manager.
- AmeDE запускается как обычное приложение внутри Wayfire; могут быть другие окна.
- Wayfire слой НЕ модифицируется AmeOS; это стандартная сборка FreeBSD pkg.

## Этапы

### 1. Wayland-сессия скрипт

`/usr/local/bin/amede-session` уже подготовлен в overlay:
- создаёт безопасный `XDG_RUNTIME_DIR` с правами `0700`, если его нет;
- сохраняет нативный Wayland-сеанс (не подставляет `DISPLAY` автоматически);
- принимает `WAYFIRE_CONFIG_FILE` для тестов/пользовательской настройки;
- проверяет наличие `wayfire` в `PATH`;
- запускает `wayfire -c <config>` через `exec`.

Wayfire потом запускает AmeDE как дочерний процесс через `[autostart]` в конфиге.

### 2. Wayfire конфиг для AmeDE

`overlay/etc/xdg/wayfire/wayfire-amede.ini`:
- Минимальный конфиг с плагинами `autostart`, `move` и `resize`
- Автоопределение outputs — без жёсткого DP-1/HDMI имени
- `xwayland = true` для X11-совместимости
- Autostart AmeDE при запуске Wayfire
- Stock panel/shell не используется

### 3. XDG session registration

`overlay/usr/local/share/wayland-sessions/amede.desktop`:
```ini
[Desktop Entry]
Name=AmeDE
Comment=AmeOS desktop environment
Exec=amede-session
Type=Application
DesktopNames=AmeDE
```

Позволяет display manager (например, `sddm` на FreeBSD) предложить эту сессию.

### 4. Desktop profile deps

`desktop-amd64.conf` дополнить списком пакетов (если будут фиксированы в дальнейшем).
Пока пакеты не управляются сборочным скриптом — `pkg install` на готовой системе.

## Проверка на FreeBSD-хосте

### Минимальная проверка (на VM без GPU)

```sh
# На готовой AmeOS desktop-системе
sudo pkg install -y wayland libxkbcommon libpixman wayfire mesa-libs xwayland foot
# Запустить в Xvfb (как в текущих тестах AmeDE)
Xvfb :99 -screen 0 1024x768x24 &
export DISPLAY=:99
amede-session
# В другом терминале: xdotool или мышь для проверки AmeDE → Wayfire взаимодействия
```

### Полная проверка (GPU-хост)

```sh
# На физ хосте или VM с GPU
startx amede-session
# Проверить: AmeDE окно появляется, мышь/клавиатура работают,
# F2 открывает меню, T запускает terminal
```

## Документация

- `ameos/docs/desktop.md` — для пользователей (как выбрать сессию)
- `ameos/docs/wayfire-setup.md` — для разработчиков (конфиг, модули, ограничения)
- `ARCHITECTURE.md` — update: что именно Wayfire делает, AmeDE делает

## Открытые вопросы

1. **Панель и док за нас:** wf-shell, Waybar или AmeDE собственная? Пока AmeDE
   рисует UI сам, Wayfire никакого UI не добавляет. Решение отложено до полной
   интеграции.

2. **Multi-monitor, дробное масштабирование:** Wayfire поддерживает; проверка
   только на физ хосте.

3. **Система уведомлений:** мint-udev → systemd-logind (нет на FreeBSD, но
   есть localseat), gnome-settings-daemon (нет), либо свой. Отложено.

4. **Горячие клавиши:** пока только AmeDE shortcuts (F2, T); системные шорткаты
   Wayfire живут в конфиге и требуют исходников.

5. **Автозапуск приложений:** через `.desktop` файлы в XDG_AUTOSTART_DIRS.

## Коммиты, что нужны

1. `wayfire-amede.ini` в overlay
2. `amede-session` скрипт
3. `amede.desktop` в overlay
4. `docs/desktop.md` — инструкция для пользователя
5. `docs/wayfire-setup.md` — для разработчиков
6. Тест apply-overlay для desktop-файлов (проверка que они на месте и не сломаны)

## Проверки без GPU

`ameos/scripts/check-amede-session.sh` проверяет наличие и базовую структуру
трёх файлов сессии, активность `autostart`, `xwayland`, отсутствие жёсткого
имени output и запуск AmeDE. `ameos/tests/amede-session.test.mjs` дополнительно
запускает сессию с fake `wayfire`: проверяет переданный config path, права
`XDG_RUNTIME_DIR` и явную ошибку при отсутствии Wayfire. Эти проверки не
доказывают, что compositor работает на реальном GPU.

## Интеграция в build-release.sh

desktop-профиль пока не ставит Wayfire-пакеты автоматически. Нужна отдельная
работа над `release_make()` логикой pkg или post-install скриптом образа.
Временно пользователь устанавливает `pkg install wayfire mesa-libs xwayland foot`
на готовой системе.

На FreeBSD-хосте с GPU остаётся обязательная проверка compositor и AmeDE.
