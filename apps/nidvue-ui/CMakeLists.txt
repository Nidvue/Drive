# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Nidvue
cmake_minimum_required(VERSION 3.21)
project(nidvue-ui VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2)
find_package(PkgConfig REQUIRED)
pkg_check_modules(FUSE3 REQUIRED fuse3)
qt_standard_project_setup()

# The Rust engine, linked as a static library. `cargo xtask ui` builds it first.
set(NIDVUE_FFI_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libnidvue_ffi.a"
    CACHE FILEPATH "Path to libnidvue_ffi.a")

if(NOT EXISTS "${NIDVUE_FFI_LIB}")
    message(FATAL_ERROR
        "libnidvue_ffi.a not found at ${NIDVUE_FFI_LIB}. Run `cargo build --release -p nidvue-ffi` first.")
endif()

qt_add_executable(nidvue-ui
    main.cpp
    bridge.cpp
    bridge.h
)

qt_add_resources(nidvue-ui "qml"
    PREFIX "/"
    FILES
        qml/Main.qml
        qml/Dashboard.qml
        qml/Conflicts.qml
        qml/Settings.qml
        qml/Logs.qml
)

target_include_directories(nidvue-ui PRIVATE
    ${FUSE3_INCLUDE_DIRS}
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../../crates/nidvue-ffi/include
)

target_compile_definitions(nidvue-ui PRIVATE NIDVUE_UI_VERSION="${PROJECT_VERSION}")

target_link_libraries(nidvue-ui PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
    ${NIDVUE_FFI_LIB}
    ${FUSE3_LIBRARIES}
    pthread
    dl
    m
)

install(TARGETS nidvue-ui RUNTIME DESTINATION bin)
