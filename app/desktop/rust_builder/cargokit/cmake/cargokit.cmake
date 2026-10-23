# Resolve `path` through a Flutter plugin symlink or junction.
# Four `..` segments from the returned directory reach the repo root
# both for rust_builder/{linux,windows,cargokit} and for the physical
# directory those names point at. A candidate that still sits under
# ephemeral/ is rejected when the repo crate is missing there.
function(cbdesk_resolve_dir out_var path)
    set(_candidates "")
    if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.19")
        file(REAL_PATH "${path}" _real)
        string(REGEX REPLACE "^\\\\\\\\[?]\\\\" "" _real "${_real}")
        list(APPEND _candidates "${_real}")
    endif()
    get_filename_component(_legacy "${path}" REALPATH)
    list(APPEND _candidates "${_legacy}" "${path}")
    if(WIN32)
        if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.17")
            set(_ps1 "${CMAKE_CURRENT_FUNCTION_LIST_DIR}/resolve_symlinks.ps1")
        else()
            set(_ps1 "${CMAKE_CURRENT_LIST_DIR}/resolve_symlinks.ps1")
        endif()
        execute_process(
            COMMAND powershell -NoProfile -ExecutionPolicy Bypass -File "${_ps1}" "${path}"
            OUTPUT_VARIABLE _ps
            ERROR_VARIABLE _ps_err
            OUTPUT_STRIP_TRAILING_WHITESPACE
            RESULT_VARIABLE _ps_rc
        )
        if(_ps_rc EQUAL 0 AND NOT _ps STREQUAL "")
            list(APPEND _candidates "${_ps}")
        endif()
    endif()
    set(_picked "")
    foreach(_candidate IN LISTS _candidates)
        get_filename_component(_crate "${_candidate}/../../../../crates/cbdesk-ffi/Cargo.toml" ABSOLUTE)
        if(EXISTS "${_crate}")
            set(_picked "${_candidate}")
            break()
        endif()
    endforeach()
    if(_picked STREQUAL "")
        list(GET _candidates 0 _picked)
    endif()
    set("${out_var}" "${_picked}" PARENT_SCOPE)
endfunction()

SET(cargokit_cmake_root "${CMAKE_CURRENT_LIST_DIR}/..")

# Workaround for https://github.com/dart-lang/pub/issues/4010
# file(REAL_PATH) resolves the Flutter plugin symlink. The older
# get_filename_component(REALPATH) leaves that link in place on Windows,
# and a failed PowerShell print must not replace the directory with empty
# (cmake -E env then fails to start run_build_tool.cmd and MSBuild reports -1).
cbdesk_resolve_dir(cargokit_cmake_root "${cargokit_cmake_root}")

# Arguments
# - target: CMAKE target to which rust library is linked
# - manifest_dir: relative path from current folder to directory containing cargo manifest
# - lib_name: cargo package name
# - any_symbol_name: name of any exported symbol from the library.
#                    used on windows to force linking with library.
function(apply_cargokit target manifest_dir lib_name any_symbol_name)
    # An absolute manifest dir is used as-is. Windows includes this file
    # through a symlink, so a relative path from CMAKE_CURRENT_SOURCE_DIR
    # would climb the link instead of the crate.
    if(IS_ABSOLUTE "${manifest_dir}")
        set(_cargokit_manifest_dir "${manifest_dir}")
    else()
        set(_cargokit_manifest_dir "${CMAKE_CURRENT_SOURCE_DIR}/${manifest_dir}")
    endif()

    set(CARGOKIT_LIB_NAME "${lib_name}")
    set(CARGOKIT_LIB_FULL_NAME "${CMAKE_SHARED_MODULE_PREFIX}${CARGOKIT_LIB_NAME}${CMAKE_SHARED_MODULE_SUFFIX}")
    if (CMAKE_CONFIGURATION_TYPES)
        set(CARGOKIT_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>")
        set(OUTPUT_LIB "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/${CARGOKIT_LIB_FULL_NAME}")
    else()
        set(CARGOKIT_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}")
        set(OUTPUT_LIB "${CMAKE_CURRENT_BINARY_DIR}/${CARGOKIT_LIB_FULL_NAME}")
    endif()
    set(CARGOKIT_TEMP_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargokit_build")

    if (FLUTTER_TARGET_PLATFORM)
        set(CARGOKIT_TARGET_PLATFORM "${FLUTTER_TARGET_PLATFORM}")
    else()
        set(CARGOKIT_TARGET_PLATFORM "windows-x64")
    endif()

    set(CARGOKIT_ENV
        "CARGOKIT_CMAKE=${CMAKE_COMMAND}"
        "CARGOKIT_CONFIGURATION=$<CONFIG>"
        "CARGOKIT_MANIFEST_DIR=${_cargokit_manifest_dir}"
        "CARGOKIT_TARGET_TEMP_DIR=${CARGOKIT_TEMP_DIR}"
        "CARGOKIT_OUTPUT_DIR=${CARGOKIT_OUTPUT_DIR}"
        "CARGOKIT_TARGET_PLATFORM=${CARGOKIT_TARGET_PLATFORM}"
        "CARGOKIT_TOOL_TEMP_DIR=${CARGOKIT_TEMP_DIR}/tool"
        "CARGOKIT_ROOT_PROJECT_DIR=${CMAKE_SOURCE_DIR}"
    )

    if (WIN32)
        set(SCRIPT_EXTENSION ".cmd")
        set(IMPORT_LIB_EXTENSION ".lib")
        # CreateProcess cannot start a .cmd. Without cmd.exe, cmake -E env
        # returns -1 and MSBuild reports MSB8066 with no Cargo output.
        set(_cargokit_tool cmd /C "${cargokit_cmake_root}/run_build_tool.cmd" build-cmake)
    else()
        set(SCRIPT_EXTENSION ".sh")
        set(IMPORT_LIB_EXTENSION "")
        set(_cargokit_tool "${cargokit_cmake_root}/run_build_tool${SCRIPT_EXTENSION}" build-cmake)
        execute_process(COMMAND chmod +x "${cargokit_cmake_root}/run_build_tool${SCRIPT_EXTENSION}")
    endif()

    # Using generators in custom command is only supported in CMake 3.20+
    if (CMAKE_CONFIGURATION_TYPES AND ${CMAKE_VERSION} VERSION_LESS "3.20.0")
        foreach(CONFIG IN LISTS CMAKE_CONFIGURATION_TYPES)
            add_custom_command(
                OUTPUT
                "${CMAKE_CURRENT_BINARY_DIR}/${CONFIG}/${CARGOKIT_LIB_FULL_NAME}"
                "${CMAKE_CURRENT_BINARY_DIR}/_phony_"
                COMMAND ${CMAKE_COMMAND} -E env ${CARGOKIT_ENV}
                ${_cargokit_tool}
                VERBATIM
            )
        endforeach()
    else()
        add_custom_command(
            OUTPUT
            ${OUTPUT_LIB}
            "${CMAKE_CURRENT_BINARY_DIR}/_phony_"
            COMMAND ${CMAKE_COMMAND} -E env ${CARGOKIT_ENV}
            ${_cargokit_tool}
            VERBATIM
        )
    endif()


    set_source_files_properties("${CMAKE_CURRENT_BINARY_DIR}/_phony_" PROPERTIES SYMBOLIC TRUE)

    if (TARGET ${target})
        # If we have actual cmake target provided create target and make existing
        # target depend on it
        add_custom_target("${target}_cargokit" DEPENDS ${OUTPUT_LIB})
        add_dependencies("${target}" "${target}_cargokit")
        target_link_libraries("${target}" PRIVATE "${OUTPUT_LIB}${IMPORT_LIB_EXTENSION}")
        if(WIN32)
            target_link_options(${target} PRIVATE "/INCLUDE:${any_symbol_name}")
        endif()
    else()
        # Otherwise (FFI) just use ALL to force building always
        add_custom_target("${target}_cargokit" ALL DEPENDS ${OUTPUT_LIB})
    endif()

    # Allow adding the output library to plugin bundled libraries
    set("${target}_cargokit_lib" ${OUTPUT_LIB} PARENT_SCOPE)

endfunction()
