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
    - [x] XOR
    - [x] NAND
    - [ ] NOR
    - [x] MUX
    - [x] POINT
    - [x] D-type flip-flops
    - [x] TEXT_FIELD
* [ ] Add component parameterization settings.
* Add SVG export:
    - [x] For connections (Net).
    - [x] For primitives.
    - [ ] For custom blocks (Unit).
* Fix bugs:
    - [ ] Bug in Net construction.
    - [ ] Bug when rotating a component with connected Nets.
* In the distant future:
    - [ ] Add support for comments in Markdown format.
