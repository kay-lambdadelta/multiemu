# Multiemu

This is a prototype of a multisystem hardware emulator, designed for fast, portable, and convenient execution of ROM based hardware systems

## Dependencies

`multiemu-utils`

| Distro    | Development Package Name |
| --------  | -------                  |
| Debian    | libbz2-dev               |

`multiemu-gui`

| Distro    | Development Package Name                                                                         |
| --------  | -------                                                                                          |
| Debian    | libx11-dev libxkbcommon-dev libwayland-dev libasound2-dev libudev-dev pkg-config build-essential |

Feature specific dependencies (all features are enabled by default)

| Distro   | Feature | Development Package Name |
| -------- | ------- | -------                  |
| Debian   | `vulkan` | libvulkan-dev           |

## MSRV

Everything in this workspace will maintain the MSRV of debian sid. It will most likely compile with a few versions down, but this program is only actively tested on debian sid.
