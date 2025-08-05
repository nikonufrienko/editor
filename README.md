## Schematic Editor

Dark theme:

![](assets/common/example_dark.jpg)

Light theme:

![](assets/common/example_light.jpg)

Key features:
* Editing schematics and saving them to a JSON file.
* Loading schematics from a JSON file.
* Exporting schematics to SVG.

## Component Types

|Type|Available Actions|
|-|-|
|Primitives|Rotation, customization (modifying various parameters)|
|Custom Blocks|Adding, renaming, and deleting ports|
|Text Fields|Editing|

## Supported Primitives
|Components|Status|Customizable Parameters|
|-|-|-|
|Logic Gates (AND, OR, XOR, NAND)|✅| Number of inputs|
|Inverter (NOT) |✅| -|
|Multiplexers|✅|Number of inputs|
|Demultiplexers|🔄|Number of outputs|
|D-type flip-flop|✅|Presence of reset ports and their polarity, presence of enable input|
|Point|✅|-|
|Input and Output|✅|-|
|Comparator (<, <=, >, >=, ==)|🔄|Comparison operation type|
|Half Adder|🔄|-|
|Full Adder|🔄|-|

## Supported Platforms
|Platform|Status|
|-|-|
|WASM|✅|
|Linux|✅|
|Windows|✅|
|Android|🔄|

## TODO List:
* [ ] Improve Net construction. Implement smart construction patterns.
* [ ] Add functionality to split Net into two parts with a new Point.
* [ ] Add more components.
* [ ] Add a "Focus on entire field" button.
