# Circuit Editor

Main features:
* Editing a circuit and saving it to a JSON file.
* Loading a circuit from a JSON file.
* Exporting a circuit to SVG.

## Supported platforms:
|Platform|Status|
|-|-|
|WASM|✅ Supported|
|Linux|✅ Supported|
|Windows|✅ Supported|
|Android|🔄 Planned|

# TODO:
* Add primitives:
    - [x] AND
    - [x] OR
    - [x] NOT
    - [ ] XOR
    - [ ] NAND
    - [ ] NOR
    - [ ] MUX
    - [ ] MUX
* [ ] Add component parameterization settings.
* Add SVG export:
    - [x] For connections (Net).
    - [x] For primitives.
    - [ ] For custom blocks (Unit).
* Fix bugs:
    - [ ] Bug in Net construction.
    - [ ] Bug when rotating a component with connected Nets.
