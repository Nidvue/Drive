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

# The translation is compiled at configure time: the .qm has to exist before the
# resource is compiled, and qt_add_translations (Qt 6.7+) is not available on
# every distribution this project builds on.
# lrelease lives in the Qt tools directory, which is not always on PATH: Ubuntu
# installs it to /usr/lib/qt6/bin from qt6-l10n-tools, nixpkgs to the qttools
# package. Search PATH first, then every place Qt itself reports.
set(NIDVUE_LRELEASE_CANDIDATES lrelease /usr/lib/qt6/bin/lrelease)
foreach(qt_dir IN ITEMS "${Qt6Core_TOOLS_DIR}" "${Qt6Core_DIR}")
    if(qt_dir)
        list(APPEND NIDVUE_LRELEASE_CANDIDATES "${qt_dir}/lrelease")
    endif()
endforeach()
find_program(LRELEASE NAMES ${NIDVUE_LRELEASE_CANDIDATES})
if(NOT LRELEASE)
    message(FATAL_ERROR
        "lrelease not found (tried ${NIDVUE_LRELEASE_CANDIDATES}): install Qt Linguist "
        "tools (qt6-l10n-tools on Debian/Ubuntu, qttools in nixpkgs) to build the UI")
endif()
message(STATUS "lrelease: ${LRELEASE}")
set(NIDVUE_I18N_DIR "${CMAKE_CURRENT_BINARY_DIR}/i18n")
file(MAKE_DIRECTORY "${NIDVUE_I18N_DIR}")
set(NIDVUE_I18N_TS "${CMAKE_CURRENT_SOURCE_DIR}/i18n/nidvue_de.ts")
execute_process(
    COMMAND "${LRELEASE}" "${NIDVUE_I18N_TS}" -qm "${NIDVUE_I18N_DIR}/nidvue_de.qm"
    RESULT_VARIABLE NIDVUE_LRELEASE_RESULT
    OUTPUT_QUIET)
if(NOT NIDVUE_LRELEASE_RESULT EQUAL 0)
    message(FATAL_ERROR "lrelease failed on ${NIDVUE_I18N_TS}")
endif()

# A generated file is an absolute path, so the resource needs an explicit alias:
# without it Qt refuses to compile the resource at all.
set_source_files_properties("${NIDVUE_I18N_DIR}/nidvue_de.qm"
    PROPERTIES QT_RESOURCE_ALIAS "nidvue_de.qm")
qt_add_resources(nidvue-ui "i18n"
    PREFIX "/i18n"
    FILES "${NIDVUE_I18N_DIR}/nidvue_de.qm"
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
