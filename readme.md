# MultiEMU

This is a prototype of a multisystem hardware emulator, designed for fast, portable, and convenient execution of ROM based hardware systems

## Dependencies

`multiemu-utils`

| Distro    | Development Package Name |
| --------  | -------                  |
| Debian    | libbz2-dev               |

`multiemu-shell-destinezite`

| Distro    | Development Package Name                                                                         |
| --------  | -------                                                                                          |
| Debian    | libx11-dev libxkbcommon-dev libwayland-dev libasound2-dev libudev-dev pkg-config build-essential |

Feature specific dependencies (all features are enabled by default)

| Distro   | Feature | Development Package Name |
| -------- | ------- | -------                  |
| Debian   | `vulkan` | libvulkan-dev           |

## UI

Note that the application being EGUI based is most likely a temporary arrangement, its just for prototyping because EGUI is the least involved when writing a renderer

## System Support

The emulator has a operational chip8 machine, along with a half finished atari 2600 and a NES machine. Planned beyond those is the Gameboy and other intel 8080/z80 based consoles.

## Accuracy Goals

The emulators systems intend to be as accurate as required for user observable state and deterministic execution (where this applies). It is not ment to replicate meaningless hardware behavior with no side effects or obscure normally unobservable behavior if that would severely affect performance and complexity.