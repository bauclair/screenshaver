# Screenshaver Maker `.smmod` Module Format

**Specification status:** Describes the supplied Screenshaver Maker Rust implementation dated November 21, 2026. **Schema version:** `1`.

A `.smmod` file is a UTF-8 JSON document defining one Maker graph module. This document is a file-format and implementation-compatibility reference, **not** a module-authoring tutorial or a guarantee that arbitrary GLSL will compile. JSON comments (`//` or `/* ... */`) and trailing commas are not supported.

## Installation, origin, and identity

Official modules are embedded into Screenshaver at build time. Local modules are discovered recursively under:

```text
~/.config/screenshaver/maker/modules/
```

The loader scans `.smmod` files to a maximum recursion depth of eight directory levels, in sorted path order. Local files receive `Community` trust regardless of any metadata in the JSON; official files receive `Official` trust. `trust` is **not** a JSON field. A local module cannot use the reserved `screenshaver` or `screenshaver.*` module ID namespace. Duplicate module IDs are rejected after the first registered definition; a malformed file is reported in the library's error collection without preventing other files from loading. The library is initialized once per process, so changing files generally requires restarting Maker.

The **Third-Party** Module Library category is a presentation policy for community modules in the accompanying Third-Party category update. The module's own `category` string remains part of its metadata; it is not a verified description of its GLSL behavior. The original uploaded loader, before that update, derived presentation categories from parent directory names. Third-party tooltips display creator attribution and a third-party disclaimer.

## Top-level JSON members

| Member | JSON type | Requirement and meaning |
|---|---|---|
| `schema_version` | integer | **Required**, exactly `1`. |
| `id` | string | **Required**, nonempty; lowercase ASCII letters, digits, `.`, `_`, `-`; must contain at least one period. Community IDs cannot begin `screenshaver.`. |
| `name` | string | **Required**, nonempty display name. |
| `category` | string | **Required**, nonempty metadata string. No fixed set of category values is enforced by the parser. |
| `created_by` | string | **Required**, nonempty creator attribution. |
| `attributed_to` | array of strings | Optional; additional attribution. Each entry must be nonempty and single-line. A legacy multiline string is also accepted; `null` or absence yields an empty list. |
| `description` | string | **Required**, nonempty description. |
| `uses` | string | **Required**, nonempty typical-uses description. |
| `notes` | string | Optional; descriptive notes, default empty. `null` is also accepted. |
| `header_color` | string | **Required**, `#RRGGBB` hexadecimal color. |
| `inputs` | array | **Required**; zero to eight input port objects. |
| `outputs` | array | **Required**; normally one to eight output port objects. A terminal module has none. |
| `parameters` | array | **Required**, may be empty. |
| `glsl` | string | **Required** for terminal and single-output modules; nonempty GLSL expression/body interpreted by the compiler. For multi-output modules, use `glsl_outputs`. |
| `glsl_outputs` | object | **Required for multi-output modules**; maps every declared output port ID to a nonempty GLSL expression string. |
| `helpers` | string | Optional GLSL helper source, default empty. |
| `terminal` | boolean | Optional, defaults `false`. Special output-terminal shape; not a normal community module pattern. |

The loader rejects a module definition longer than **131,072 bytes** (128 KiB), although it parses JSON before applying this size check. Unrecognized top-level members are not explicitly rejected by the current parser, but are not preserved in the in-memory `Module` structure.

## Port objects (`inputs` and `outputs`)

| Member | Type | Meaning |
|---|---|---|
| `id` | string | **Required**, nonempty GLSL-style identifier: first character ASCII letter or `_`, remaining ASCII letters, digits, or `_`. |
| `name` | string | **Required**, nonempty display label. |
| `type` | string | **Required**; one of `float`, `vec2`, `vec3`, `field2d`, `field3d`. |
| `default` | string | Optional; raw GLSL expression used when an input is unconnected. Absence defaults to `""`, which can mean a required connection. |
| `semantic_role` | string | Optional; one of the recognized roles below, or empty. |

Allowed `semantic_role` values:

```text
uv, position2d, offset2d, direction2d, position3d,
direction3d, normal3d, color, distance, time, angle,
mask, intensity, curvature, occlusion
```

Port IDs must be unique within the inputs array and separately within the outputs array. The parser does **not** enforce uniqueness across the two arrays. The compiler and graph can use semantic roles to interpret coordinate inputs and callable sampling behavior; they are not merely cosmetic labels.

**Callable-field restriction:** Community modules may declare `field2d` and `field3d` **inputs**, but cannot directly declare these types as outputs. The loader reserves explicit callable-field outputs for the built-in `screenshaver.test.universal_test` module. Maker can derive callable outputs from eligible position-dependent scalar modules; this depends on their declared inputs, outputs, and graph/compiler behavior, not on a JSON output declaration.

## Parameter objects (`parameters`)

| Member | Type | Meaning |
|---|---|---|
| `id` | string | **Required** GLSL-style identifier; unique within `parameters`. |
| `name` | string | **Required** nonempty display name. |
| `type` | string | **Required**; `float`, `color3`, or `choice`. |
| `default` | number or array | **Required**. `float` and `choice` use a number; `color3` uses an array of exactly three numbers. |
| `min` | number | Optional, defaults to `0.0`. |
| `max` | number | Optional, defaults to `1.0`. Must be greater than or equal to `min`. |
| `options` | array of strings | Required for `choice`: between 2 and 16 nonempty labels. |

A `choice` default must be an integer-valued numeric index from `0` through `options.length - 1`. The parser does not require a `float` default to lie between `min` and `max`. A `color3` parameter is emitted as a GLSL `vec3` uniform; `float` and `choice` parameters are emitted as `float` uniforms. Parameters are per graph-node instance.

## GLSL expressions, substitutions, and helpers

The compiler generates a GLSL `#version 330 core` fragment shader around the connected graph. Module expressions are **not** complete standalone fragment shaders. The generated shader provides `u_resolution` (`vec2`), `u_time` (`float`), and per-node parameter uniforms named `u_n<node_id>_<parameter_id>`.

An expression may refer to declared input and parameter IDs using **`{identifier}`** placeholders. The compiler replaces each placeholder with the corresponding connected value, input default, or parameter uniform reference. Unknown placeholders, an unclosed `{`, or an unexpected `}` cause generation errors. This substitution is textual; it is not a general GLSL parser. Literal braces in expression templates are therefore problematic. The `helpers` field is emitted as raw GLSL source, separately from placeholder substitution; its content must be valid in the generated shader's context. Helper code is emitted once per distinct helper-source string.

For a single-output nonterminal module, `glsl` supplies the output expression. For a multi-output module, `glsl_outputs` must contain an expression for **each** declared output ID. The first output expression also becomes the internal `glsl` value. The loader rejects `#version`, `#include`, or `void main` in output expressions / the internal `glsl` string. **This check does not cover the `helpers` string**, and is not a complete GLSL safety or semantic validator. Actual GLSL compilation occurs when a graph is compiled for preview/rendering.

### Graph and compiler compatibility constraints

A module passing JSON validation is **not** guaranteed to produce a valid shader. Graph connection types, required inputs, callable-field compatibility, expression substitutions, helper naming, and GLSL compilation impose additional constraints. Some compiler branches are specialized for specific built-in `screenshaver.*` module IDs; community modules should not assume those behaviors are available to arbitrary IDs. A nonterminal module must have at least one output; at most eight are supported for community modules. The parser's special ten-output exception applies only to the built-in universal-test ID.

A `terminal` module must have exactly one `vec3` input and zero outputs. Maker's graph uses a designated built-in output terminal, and community modules cannot impersonate its reserved ID. Consequently `terminal: true` is not a generally useful third-party extension point.

## Example: complete third-party `.smmod` file

The following is a generic, valid JSON example of a single-output color-adjustment module. It illustrates the file structure; it is not a complete shader and does not require a `main()` function. The example ID uses a nonreserved namespace.

```json
{
  "schema_version": 1,
  "id": "example.color.brightness_adjust",
  "name": "Brightness Adjust",
  "category": "color",
  "created_by": "Example Developer",
  "attributed_to": [],
  "description": "Scales an input RGB color by an adjustable brightness factor.",
  "uses": "Connect a vec3 color source to adjust its brightness.",
  "notes": "Generic example of a single-output, nonterminal Maker module.",
  "header_color": "#477DB3",
  "inputs": [
    {
      "id": "input_color",
      "name": "Input Color",
      "type": "vec3",
      "semantic_role": "color",
      "default": "vec3(1.0)"
    }
  ],
  "outputs": [
    {
      "id": "result",
      "name": "Result",
      "type": "vec3",
      "semantic_role": "color"
    }
  ],
  "parameters": [
    {
      "id": "brightness",
      "name": "Brightness",
      "type": "float",
      "default": 1.0,
      "min": 0.0,
      "max": 2.0
    }
  ],
  "glsl": "{input_color} * {brightness}",
  "helpers": "",
  "terminal": false
}
```

The `inputs` array declares a `vec3` color input, and its `default` supplies a GLSL value when that input is unconnected. The `parameters` array declares an adjustable brightness value. In `glsl`, `{input_color}` refers to the input and `{brightness}` refers to the parameter; Maker substitutes their expressions or uniform references when compiling the graph. The output is a `vec3` suitable for connection to other compatible modules.

For this local module, the library assigns community trust and displays it in **Third-Party**, regardless of the illustrative `"category": "color"` metadata. The `created_by` value is displayed as author attribution.

## Validation and diagnostic behavior

Loading validates JSON syntax, schema version, identity/namespace, color formatting, port and parameter definitions, terminal/output structure, expression presence, module size, and limited forbidden GLSL tokens. Errors are collected with the source file path. Successful registration does not establish that the module's declared category matches its behavior, that its GLSL is correct, that its helpers are collision-free, or that it is safe or appropriately licensed.

The loader does not automatically verify authorship or licensing. `created_by` and `attributed_to` are attribution metadata supplied by the module author, not a license grant or an authenticity check. Community code remains third-party regardless of these values.

## Implementation references

This specification is derived from the supplied source files:

- `maker_module.rs`: JSON parsing, supported members, types, and validation.
- `maker_library.rs`: embedded official catalog, community discovery, namespace/duplicate handling, trust, and category registration.
- `maker_compiler.rs`: GLSL expression substitution, helper emission, and callable generation.
- `maker_graph.rs`: graph connectivity and inferred callable-field interfaces.
- `maker_ui.rs`: Module Library grouping and attribution/tooltips.

**Compatibility note:** This document describes the inspected November 21, 2026 implementation, including the separately prepared Third-Party category update where explicitly identified. Future changes to the parser or compiler may change supported syntax or behavior; the implementation remains authoritative.
