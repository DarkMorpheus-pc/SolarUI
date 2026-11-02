# BlazeOS / SolarUI changes

This is the vendored source of end4-pC used for the Niri-based SolarUI integration.

## Integration policy

- Preserve the end4-pC name, copyright notices, credits and GPL-3.0 license.
- Replace or isolate compositor-specific Hyprland paths when running under Niri.
- Replace Arch-only package operations with distribution-aware adapters.
- Keep program files read-only at runtime and store user configuration/state under XDG paths.
- Do not expose controls whose implementation or optional dependency is unavailable.
- Launch and stop this shell only through the SolarUI supervisor with health checks and rollback.

## 2026-11-02

- Upgraded upstream end4-pC from `8fbb6716cba628a13103e665e9538021683a7250` to `e29674b92bc4dc25d8a17a8c503a7c83f7922fd1` (113 upstream commits including full Niri layout/session/cliphist/screenshot support, pauseable live video wallpapers via IPC sockets, multi-GPU renderer detection, and atomic JSON config writes).
- Rebased BlazeOS / SolarUI integration patches onto `e29674b`, adding `portadora` package manager support in `services/Updates.qml` and `services/SystemInfo.qml` and Niri monitor detection in `scripts/colors/switchwall.sh`.

## 2026-10-16

- Imported upstream commit `8fbb6716cba628a13103e665e9538021683a7250` without functional modifications.
- Began the Niri compatibility and SolarUI provider audit.
- Disabled Hyprland autostart and conflict-killer execution in Niri sessions.
- Removed the unsafe unknown-compositor fallback to the Hyprland backend.
- Added a SolarUI IPC readiness probe for supervisor health checks.
- Replaced Arch-only update counting and package statistics with Fedora/DNF/RPM-aware fallbacks.
- Replaced KDE/Plasma application defaults with BlazeOS-compatible NetworkManager, Blueman, htop and pavucontrol commands.
- Removed the hard QtPositioning startup dependency; weather falls back to the configured city until portal/GeoClue location support is implemented.
- Removed the KDE syntax-highlighting runtime dependency from AI code blocks; language labels, editing, copying and saving remain available.
- Replaced the Kirigami-only application icon widget with a native Qt Quick image widget.
