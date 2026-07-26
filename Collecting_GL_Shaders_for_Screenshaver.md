# Collecting GL Shaders for Screenshaver

## Overview

Screenshaver supports two of the most popular GLSL shader ecosystems:

-   **ShaderToy** shaders (saved with the `.glsl` extension)
-   **ISF (Interactive Shader Format)** shaders (saved with the `.fs`
    extension)

This guide describes a recommended workflow for building a high-quality
local shader collection.

------------------------------------------------------------------------

## Step 1 -- Browse Shader Libraries

### ShaderToy

Visit:

-   https://www.shadertoy.com/

Browse the available shaders using the site's categories, popularity
lists, search feature, or newest uploads.

> **Important:** Screenshaver supports **only ShaderToy shaders that
> consist of a single `Image` tab** on the shader's code page.
>
> **Do not download** ShaderToy shaders that contain:
>
> -   `Common`
> -   `Buffer A`
> -   `Buffer B`
> -   `Buffer C`
> -   `Buffer D`
> -   `Cube A`
> -   Any other additional rendering passes
>
> These are multi-pass shaders that require multiple rendering buffers
> and are **not compatible with Screenshaver**. Only single-pass shaders
> with a single **Image** tab should be collected.

### ISF

Visit:

-   https://editor.isf.video/

Browse the available Interactive Shader Format examples and community
shaders.

------------------------------------------------------------------------

## Step 2 -- Copy the Shader Source

When you find a compatible shader you would like to test:

1.  Open the shader.
2.  Verify that it contains **only a single `Image` tab** (ShaderToy
    only).
3.  Locate the GLSL source code.
4.  Copy the fragment shader source.

For ShaderToy:

Save the file using the extension:

``` text
MyShader.glsl
```

For ISF:

Save the file using the extension:

``` text
MyShader.fs
```

Screenshaver automatically detects the shader type from its contents and
file extension.

------------------------------------------------------------------------

## Step 3 -- Create a Test Collection

Create a temporary directory for evaluation.

``` bash
mkdir -p ~/Downloads/TestShaders
```

Copy your downloaded shader files into this directory.

------------------------------------------------------------------------

## Step 4 -- Preview the Shaders

Use Screenshaver's preview mode to automatically cycle through every
shader.

``` bash
screenshaver --preview-shader ~/Downloads/TestShaders --interval 10
```

or

``` bash
screenshaver --preview-shader /path/to/shaders/ --interval 10
```

This command:

-   loads every shader in the directory
-   displays each shader for 10 seconds
-   automatically advances to the next shader
-   records compatibility information in the Screenshaver log (when
    enabled)

------------------------------------------------------------------------

## Step 5 -- Evaluate Each Shader

As each shader is displayed, consider the following:

### Visual Quality

-   Is the shader attractive?
-   Does it animate smoothly?
-   Is it appropriate for long-term viewing?

### Compatibility

Remove shaders that:

-   fail to compile
-   render incorrectly
-   display severe artifacts
-   depend on unsupported features

### Performance

Remove shaders that:

-   consume excessive GPU resources
-   cause reduced frame rates
-   produce excessive fan noise or heat
-   perform poorly on typical hardware

Remember that a screensaver may run for many hours, so efficient shaders
are preferred over visually impressive but expensive effects.

------------------------------------------------------------------------

## Step 6 -- Build Your Certified Collection

After testing is complete, move the approved shaders into the
Screenshaver shader directory.

``` bash
mkdir -p ~/.config/screenshaver/shaders

mv ~/Downloads/TestShaders/*.glsl ~/.config/screenshaver/shaders/
mv ~/Downloads/TestShaders/*.fs   ~/.config/screenshaver/shaders/
```

Only shaders that have been tested and approved should be copied into
this directory.

------------------------------------------------------------------------

## Recommended Workflow

1.  Browse ShaderToy and ISF.
2.  For ShaderToy, select only shaders with a single **Image** tab.
3.  Copy the shader source.
4.  Save it as `.glsl` or `.fs`.
5.  Place it into a temporary test directory.
6.  Preview the directory using:

``` bash
screenshaver --preview-shader /path/to/shaders/ --interval 10
```

7.  Remove incompatible or high-GPU-cost shaders.
8.  Move the remaining certified shaders into:

``` text
~/.config/screenshaver/shaders
```

Following this workflow keeps your production shader library clean,
reliable, and efficient while making it easy to evaluate new additions
before they become part of your permanent Screenshaver collection.
