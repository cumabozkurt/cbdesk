cmake_minimum_required(VERSION 3.14)

set(PROJECT_NAME "cbdesk_ffi")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
# Flutter configures this file through windows/flutter/ephemeral/.plugin_symlinks.
# get_filename_component(REALPATH) does not resolve that Windows symlink, so
# four `..` segments look for the crate under flutter/ instead of the repo.
cbdesk_resolve_dir(_plugin_windows "${CMAKE_CURRENT_LIST_DIR}")
get_filename_component(CBDESK_FFI_CRATE "${_plugin_windows}/../../../../crates/cbdesk-ffi" ABSOLUTE)
if(NOT EXISTS "${CBDESK_FFI_CRATE}/Cargo.toml")
  message(FATAL_ERROR "cbdesk_ffi: Cargo.toml not found at ${CBDESK_FFI_CRATE} (plugin dir ${_plugin_windows})")
endif()
message(STATUS "cbdesk_ffi crate: ${CBDESK_FFI_CRATE}")
apply_cargokit(${PROJECT_NAME} "${CBDESK_FFI_CRATE}" cbdesk_ffi "")

set(cbdesk_ffi_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  PARENT_SCOPE
)
