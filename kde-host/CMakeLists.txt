cmake_minimum_required(VERSION 3.21)
project(ScreenshaverKdeHost LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

# Keep build-tree runtime paths relocatable.  The plugin is packaged beside
# libscreenshaver.so, so its private Screenshaver dependency must resolve via
# $ORIGIN rather than retaining the temporary build directory.
set(CMAKE_BUILD_RPATH_USE_ORIGIN TRUE)

find_package(Qt6 REQUIRED COMPONENTS Core DBus Gui Qml Quick)

set(
    SCREENSHAVER_RENDERER_LIBRARY
    ""
    CACHE FILEPATH
    "Path to kde-renderer libscreenshaver.so"
)

if(NOT SCREENSHAVER_RENDERER_LIBRARY)
    message(
        FATAL_ERROR
        "SCREENSHAVER_RENDERER_LIBRARY must point to kde-renderer libscreenshaver.so"
    )
endif()

if(NOT EXISTS "${SCREENSHAVER_RENDERER_LIBRARY}")
    message(
        FATAL_ERROR
        "Screenshaver renderer library does not exist: ${SCREENSHAVER_RENDERER_LIBRARY}"
    )
endif()

add_library(
    ScreenshaverNativeGLPlugin
    SHARED
    plugin.cpp
    native_opengl_underlay.cpp
    native_opengl_underlay.h
    native_opengl_renderer.cpp
    native_opengl_renderer.h
)

target_link_libraries(
    ScreenshaverNativeGLPlugin
    PRIVATE
        Qt6::Core
        Qt6::DBus
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        "${SCREENSHAVER_RENDERER_LIBRARY}"
)

set_target_properties(
    ScreenshaverNativeGLPlugin
    PROPERTIES
        LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/qml/ScreenshaverNativeGL"

        # The production package copies libscreenshaver.so beside this plugin.
        # Use the final relocatable RPATH in the build artifact itself so CMake
        # never records kde-renderer/target/... (or Nix's /build directory).
        BUILD_WITH_INSTALL_RPATH TRUE
        BUILD_RPATH "$ORIGIN"
        INSTALL_RPATH "$ORIGIN"
        INSTALL_RPATH_USE_LINK_PATH FALSE
)

add_custom_command(
    TARGET ScreenshaverNativeGLPlugin
    POST_BUILD
    COMMAND
        ${CMAKE_COMMAND}
        -E copy_if_different
        "${CMAKE_CURRENT_SOURCE_DIR}/qmldir"
        "${CMAKE_BINARY_DIR}/qml/ScreenshaverNativeGL/qmldir"
)
