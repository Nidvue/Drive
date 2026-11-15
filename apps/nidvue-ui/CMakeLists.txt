# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Nidvue
cmake_minimum_required(VERSION 3.21)
project(nidvue-ui VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.4 REQUIRED COMPONENTS Core Gui Widgets Qml Quick QuickControls2)
if(UNIX AND NOT APPLE)
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(FUSE3 REQUIRED fuse3)
endif()
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
    preferences.cpp
    preferences.h
    tray.cpp
    tray.h
)

qt_add_resources(nidvue-ui "qml"
    PREFIX "/"
    FILES
        qml/qmldir
        qml/Theme.qml
        qml/Card.qml
        qml/StatCard.qml
        qml/PillButton.qml
        qml/NavItem.qml
        qml/Toggle.qml
        qml/ToggleRow.qml
        qml/SliderRow.qml
        qml/Field.qml
        qml/SegmentedChoice.qml
        qml/Main.qml
        qml/Dashboard.qml
        qml/Conflicts.qml
        qml/Transfers.qml
        qml/Settings.qml
        qml/SettingsPane.qml
        qml/SettingsWindow.qml
        qml/Popup.qml
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
    Qt6::Widgets
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
    ${NIDVUE_FFI_LIB}
)

if(UNIX)
    target_link_libraries(nidvue-ui PRIVATE pthread dl m ${FUSE3_LIBRARIES})
endif()

# The credential store reaches the OS keyring over D-Bus on Linux, and the
# static Rust library carries that symbol, so the UI must link it too.
find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
    pkg_check_modules(DBUS QUIET dbus-1)
    if(DBUS_FOUND)
        target_link_libraries(nidvue-ui PRIVATE ${DBUS_LIBRARIES})
    endif()
endif()

install(TARGETS nidvue-ui RUNTIME DESTINATION bin)
