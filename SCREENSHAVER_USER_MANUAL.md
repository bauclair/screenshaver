1. Description

   1. Introduction to Screenshaver

      1. Screenshaver is a next-generation Linux screensaver program, programmed in Rust, that renders and displays full-screen GL shaders after a predefined amount of time without keyboard or mouse activity occurs on a user’s system. When keyboard or mouse activity again occurs, the screensaver deactivates, and restores the user desktop as it was left before screensaver engagement. The name “Screenshaver” is a tongue-in-cheek combination of the words “screensaver” and “shader”. Screenshaver can render GL shader files with both .glsl and .fs extensions. Screenshaver is currently in beta-test mode, with a full production release anticipated at version 1.0.0.

   2. What is a GL Shader?

      1. A GL shader is a text file containing mathematical formulas that are interpreted by Screenshaver and other GL viewers to display a virtually limitless variety of animated shapes and colors on the user’s monitor screen.

   3. Textures

      1. Screenshaver has a number of built-in textures that can be used with GL shaders that require them. They include:

         1. Bricks

         2. Cells

         3. Clouds

         4. Hexagons

         5. Marble

         6. Mesh

         7. Noise

         8. Radial

      2. Graphic Primitives

         1. Each Screenshaver texture has a graphic primitive associated with it. This defines the number of cells, bricks, hexagons or other shapes that are repeated on the screen as part of the texture. Screenshaver’s graphic primitives have a range from 1 through 1024, allowing great variety in the textures that are generated.

   4. Palettes

      1. Screenshaver features built-in background color palettes that can be used with GL shaders that require a texture to render properly. They include:

         1. brick

         2. bronze

         3. lichen

         4. mist

         5. sandstone

         6. slate

   5. FPS

      1. FPS stands for “frames per second”. It is the rate that individual video frames are rendered on a display monitor. Screenshaver supports rendering shaders at rates from 16 to 120 fps. Higher fps rates result in smoother animation of shaders, while lower fps rates may introduce choppiness. A reasonable starting point for fps in Screenshaver is 30fps. Please do not increase fps unless there is a compelling reason to do so-- excessive fps values risk overtaxing your GPU.

2. Features

   1. Screensaver

      1. Screenshaver’s main usage is as a desktop computer or laptop screensaver. Once started, the program waits until the amount of time defined by the “idle\_timeout” value in screenshaver.toml passes without keyboard or mouse activity. When this threshold is reached, the program begins rendering full-screen GL shaders using the display mode (Single, Random or Ordered) set in screenshaver.toml.

   2. GL Shader Viewer

      1. Screenshaver can also be used as a stand-alone GL shader viewer, regardless of whether the screensaver portion of the program is running.

   3. GL Texture Previewer

      1. Screenshaver can be used to preview combinations of textures, graphic primitives and palette colors used by GL shaders that require textures to render. Screenshaver textures are created and used in-memory only-- no actual texture files are generated.

   4. GL Shader Pre-Processing

      1. Pre-processed shader cache

         1. Screenshaver NEVER alters an original GL shader file. Instead, a “working” copy of the shader file is created in the /screenshaver/cache subfolder, with a \*.\_gen.glsl or \*.\_gen.fs file extension. Any Screenshaver pre-processing is applied to this file, not the original GL shader. This has the added benefit of ensuring that, once a file has been pre-processed and placed in the /cache subfolder, it will never need to be pre-processed again, unless the “screenshaver –delete-cache” command is run and all cache files are deleted.

   5. Variable FPS Shader Playback

      1. Screenshaver can be configured to render GL shaders at a rate from 16 to 120 frames per second. The maximum fps obtainable is totally dependent upon the user’s hardware, and care should be taken not to exceed the hardware’s capacity by running at too high a fps value.

3. Screenshaver Configuration File

   1. The Screenshaver configuration file is located in the user’s ~/.config/screenshaver/ folder.

   2. Screenshaver.toml is a text file formatted in TOML (Tom’s Obvious Minimal Language) format. It contains information that Screenshaver needs to operate correctly.

   3. Configuration Options

      1. show\_splash = true|false

         1. Determines whether or not Screenshaver will display its “splash” screen on program startup.

      2. Subtitles = true|false

         1. Determines if Screenshaver displays filename, FPS and other information for the GL shaders it renders, using a pill overlaid directly onto the active shader.

      3. subtitle\_placement = top|bottom:left|right|center

         1. Determines description pill placement on the monitor screen when Subtitles = true.

      4. Mode = Single:filename|Random:interval|Ordered:interval

         1. Determines the mode Screenshaver uses to display GL shaders.  “Single” limits the display to a single GL shader specified after “Single:”. “Random” displays any GL shader found in Screenshaver’s /shaders subfolder, rendering a new shader at random for an interval in seconds defined by the number after “Random:”. “Ordered” works the same as “Random”, but displays shaders in alphanumerical order by filename, for an interval in seconds defined by the number after “Ordered:”.

      5. idle\_timeout = nn\[s|m|h\]

         1. Determines the amount of time in seconds, minutes or hours that Screenshaver will wait without keyboard or mouse activity, before starting the screensaver and rendering a GL shader to the screen. Any mouse or keyboard activity after this period of time will deactivate the screensaver and return to the user desktop.

      6. global\_texture = \[texture\]

         1. Screenshaver’s default behavior when rendering a shader that requires a texture is to select both the texture and the number of graphic primitives at random. Users can override this randomization and define a specific combination of texture and number of graphic primitives that is used globally for all shaders that require it.

      7. global\_palette = \[palette\]

         1. Screenshaver’s default behavior when rendering a shader requiring a palette is to select the palette at random. Users can override randomization and define a specific palette that is used globally for all shaders that require it.

      8. \[\[texture\_override\]\]

         1. Texture and number of graphic primitives can be overridden on a per-shader basis by using a \[\[texture\_override\]\] block. Users can define as many of these blocks for as many individual shaders as is necessary.

      9. global\_rendered\_fps = nn

         1. Frames per second rendering speed can be defined globally for all GL shaders. The default value is 30fps, with an overall range from 16 to 120fps. WARNING: setting global\_fps to too high a value runs the risk of overtaxing your GPU, causing overheating and other undesirable issues. 30Fps is a generally sensible starting value for most users. The global fps value can only be overridden by \[\[fps\_override\]\] blocks created for specific shaders.

      10. \[\[fps\_override\]\]

          1. Frames per second rendering speed can be defined on a per-shader basis. This is generally done to accommodate specific shaders with a known high-GPU requirement.

      11. debug\_log = true|false

          1. The debug\_log item determines whether or not a runtime/debug log will be generated while the program is operating. If debug\_log = true, the old debug log is truncated and a new one is created each time the program is started.

      12. debug\_level = n

          1. Determines the amount of detail in the screenshaver.log runtime/debug log. debug\_level range is from 1 to 6. Lower numbers provide less detail, while higher numbers provide more. Debug log levels are additive-- each higher number displays all the information for its own level and all levels below it.

4. Override Hierarchies

   1. Shaders

      1. If no texture or palette information is specified, texture, number of graphic primitives, and color palette are selected randomly.

      2. If global\_texture and/or global\_palette are defined in screenshaver.toml, textures will be rendered using the global texture and/or palette values defined.

      3. If texture and/or palette are defined on a per-shader basis, those values will take precedence over global values.

      4. If texture and/or palette are defined explicitly on the –preview-shader command line, those values will take precedence over global and per-shader directives.

   2. Textures

      1. If no palette is specified, textures will be rendered with a random color palette.

      2. If a specific color palette is specified on the –preview-texture command line, it will take precedence over any active global\_palette directive.

5. System Tray Icon

   1. Screenshaver provides a system tray icon that can be used by KDE and other desktops. When this icon is visible, Screenshaver is active and running. Right-clicking on the tray icon gives the user a choice of stopping and/or restarting Screenshaver. NOTE: This tray icon may not be available on all Linux desktops or window managers.

6. Desktop Icon

   1. Screenshaver configures a desktop icon when it is installed. This icon can be created on a Linux desktop, or added to a supported desktop bar or dock. Right-clicking on the desktop icon provides options to start or stop the program (left-clicking the icon starts the program by default).

7. Single-Execution Enforcement

   1. Screenshaver can only be run in a single instance as a screensaver. Multiple screensaver instances are not supported, but the program can execute command-line options such as –preview-shader and –preview-texture while the main screensaver portion of the program is active.  

8. Command-Line Options

   1. --help

      1. Displays all of Screenshaver’s command-line options.

   2. --version

      1. Displays the version of Screenshaver the user is running.

   3. --start

      1. Starts Screenshaver. This is the same as running Screenshaver from a desktop icon, an application dock or application manager. This command-line option can be associated with a keybind in window managers like Hyprland, Mango and Niri.

   4. --stop

      1. Stops Screenshaver. This provides a way to cleanly exit Screenshaver as part of a keybind, shell script or other method.

   5. --delete-cache

      1. Deletes all files in the Screenshaver pre-processed shader cache. This forces Screenshaver to re-process original shaders and re-create entries in the cache the next time a shader is rendered.

   6. --preview-shader

      1. This command-line option allows Screenshaver to be run as a GL file viewer. The selected shader starts rendering immediately, and exits when keyboard or mouse activity is detected.

   7. --preview-texture

      1. This command-line option allows Screenshaver to be used to view different combinations of textures, graphic primitives and color palettes available for use by shaders that require textures to render properly. The texture/primitive/palette combination is displayed until keyboard or mouse activity is detected.

   8. --list-textures

      1. This command-line option lists all of the current texture categories supported by Screenshaver.

   9. --list-palettes

      1. This command-line option lists all of the color palettes currently supported by Screenshaver.

9. Logging

   1. Log levels

      1. Level 1 – Critical

         1. Logs only events that prevent Screenshaver from starting, continuing or shutting down safely.

      2. Level 2 – Errors

         1. Level 1 plus recoverable failures.

      3. Level 3 – Warnings

         1. Levels 1-2 plus suspicious or degraded behavior.

      4. Level 4 – Informational

         1. Levels 1-3 plus normal major program lifecycle events.

      5. Level 5 – Debug

         1. Levels 1-4 plus detailed troubleshooting information.

      6. Level 6 – Trace

         1. Everything.

10. Visual Notifications and Warnings

    1. FPS/Performance Warnings

       1. Regardless of whether you have defined “Subtitles = false” in Screenshaver.toml, you may see a color-coded warning pill occasionally pop up on a rendering GL shader. These are indications that your PC is having problems rendering the shader, usually due to a combination of a poorly-designed shader that takes too many resources to display and/or rendering the shader at too high a frames-per-second (FPS) rate. If the “FPS =” information is displayed in yellow, this shader is marginal for your hardware and the FPS rate you are running it at. If the “FPS =” information is flashing in bold red, you should not run this shader on your system for an extended period of time. You should either remove the shader completely, or create a \[\[fps\_override\]\] block to decrease the fps for that specific shader and see if that makes the FPS warning go away.

11. Where to Find GL Shaders

    1. Tens of thousands of GL shaders are available on the Internet, at public websites like:

       1. ShaderToy ([https://www.shadertoy.com](https://www.shadertoy.com/))

       2. ISF Video ([https://isf.video](https://isf.video/))

       3. GLSL Sandbox ([https://glslsandbox.com](https://glslsandbox.com/))

    2. To acquire a GL shader, go to one of these sites and navigate to the page where the code for the GL shader you want is displayed. Copy the entire GL code block (you can usually use CTRL-A to select it), then open a text editor on your local PC. Paste the code into the text editor, and save it using the same filename and extension as is displayed for the shader on the website. Be careful to observe any copyright or ownership restrictions that may be posted within the shader file-- you must obey these laws, and should not copy restricted shaders against the author’s wishes.

12. Best Practices: Reviewing New GL Shaders

    1. It is recommended that you review all of the GL shaders in your /screenshaver/shaders subfolder at least once, to make sure that there are no problems with GL loading, pre-processing or rendering. Screenshaver will not be compatible with all GL shaders-- some may even make the program crash. It is a best practice to run “screenshaver –preview-shader ~/.config/screenshaver/shaders/ --interval 10” on your first batch of downloaded shader files. This allows the program to quickly step through each shader file, rendering it for 10 seconds before going on to the next shader. You can use this to weed out any shaders that do not render correctly (or at all). If you have many shaders already certified in your /shaders subfolder, you may want to run screenshaver –preview-shader in a different folder, for example your home directory, instead of the /shaders subfolder. You just need to make sure that the shaders to be evaluated are located wherever you are telling Screenshaver to look for them.

13. Rejecting Incompatible GL Shaders

    1. Screenshaver has a limited ability to recognize GL shaders that it cannot pre-process and/or render. Any GL shaders in this condition are automatically removed from the /shaders subfolder and placed in a /rejected folder at the same level. In addition to the rejected shader, a text file listing the reason(s) for rejection is also created in the same folder.

14. Program Limitations

    1. Screenshaver cannot be run as root, either as a sudo command or from a user session logged in as root.

    2. Screenshaver may not fully support rendering shaders to multiple monitors. If you would like to beta test Screenshaver with a multi-monitor configuration, please send email to [screenshaver@proton.me](mailto:screenshaver@proton.me). 

15. Troubleshooting and Support

    1. If you are having problems running Screenshaver from a window manager, desktop or application manager, try opening a terminal session and running “screenshaver”. Often, messages displayed in the terminal console can be a clue to what the problem is.

    2. If the program does not run, or runs incorrectly, the screenshaver.log file can also be an invaluable source of troubleshooting information. To make sure the maximum amount of information is collected, log\_level should be dialed up to 5, then running the program should be retried to see if it produces any clues in the runtime/debug log.

    3. If the Linux distribution, desktop or window manager you are running is non-standard, it is entirely possible that Screenshaver code may need to be modified to support your specific use case. If you believe this to be the case, please run the “screenshaver-diag” shell script supplied with the Screenshaver package. This script will gather information specific to your system’s hardware, software and graphics capabilities, and generate a text file that can be mailed to [screenshaver@proton.me](mailto:screenshaver@proton.me) for review by our developers. Please open the screenshaver-diag shell script in a text editor, and review the package dependencies before attempting to run the script.

16. Feature Requests

    1. We are always receptive to new ideas coming from Screenshaver users. Please email requests for new or expanded features to [screenshaver@proton.me](mailto:screenshaver@proton.me). Our developers will review them for feasibility.

17. Beta Testing

    1. Screenshaver is currently in “beta test” mode, being actively developed, with a full production release expected for version 1.0.0. You are always welcome to contribute your skills, experience and opinions as a beta tester, in order to make Screenshaver more flexible, reliable and relatable to a wider audience of Linux users.
