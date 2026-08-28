cmake_minimum_required(VERSION 3.21)
project(ScreenshaverKdeHost LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick)

set(SCREENSHAVER_RENDERER_LIBRARY "" CACHE FILEPATH
    "Path to kde-renderer libscreenshaver.so")

if(NOT SCREENSHAVER_RENDERER_LIBRARY)
    message(FATAL_ERROR
        "SCREENSHAVER_RENDERER_LIBRARY was not specified.\n"
        "Pass -DSCREENSHAVER_RENDERER_LIBRARY=/path/to/kde-renderer/target/<profile>/libscreenshaver.so")
endif()

if(NOT EXISTS "${SCREENSHAVER_RENDERER_LIBRARY}")
    message(FATAL_ERROR
        "Screenshaver KDE renderer library not found: ${SCREENSHAVER_RENDERER_LIBRARY}")
endif()

add_library(ScreenshaverNativeGLPlugin SHARED
    plugin.cpp
    native_opengl_underlay.cpp
    native_opengl_underlay.h
    native_opengl_renderer.cpp
    native_opengl_renderer.h
)

target_include_directories(ScreenshaverNativeGLPlugin
    PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}"
)

target_link_libraries(ScreenshaverNativeGLPlugin
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        "${SCREENSHAVER_RENDERER_LIBRARY}"
)

set_target_properties(ScreenshaverNativeGLPlugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/qml/ScreenshaverNativeGL"
    BUILD_RPATH "$ORIGIN"
    INSTALL_RPATH "$ORIGIN"
)

add_custom_command(TARGET ScreenshaverNativeGLPlugin POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${CMAKE_CURRENT_SOURCE_DIR}/qmldir"
            "${CMAKE_BINARY_DIR}/qml/ScreenshaverNativeGL/qmldir"
)
