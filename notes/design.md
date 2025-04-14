# Design

This emulator orients around a machine, which is a large state machine encapsulating all required functionality and data for the emulator.

It contains:

- Components
- Various data required for runtime subsystem interaction

## Component

A component is the smallest unit of this emulator. It hooks into the global state machine, and provides functionality for other components/the runtime to interact with

Components are created using the FromConfig trait, and a set of quirks (if any, and when such a system is implemented). The runtime provides the data structures required for a component to complete its functionality while the component inserts its code into the machine and adds itself to the component store.

A component can have a set of quirks, defined per rom, that tell it to act in some kind of special cased behavior (eg: certain atari 2600 roms could have their rom mapper type manually set).
