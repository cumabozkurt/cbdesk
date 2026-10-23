cmake_minimum_required(VERSION 3.10)

set(PROJECT_NAME "cbdesk_ffi")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
get_filename_component(_real_dir "${CMAKE_CURRENT_LIST_DIR}" REALPATH)
get_filename_component(CBDESK_FFI_CRATE "${_real_dir}/../../../../crates/cbdesk-ffi" ABSOLUTE)
apply_cargokit(${PROJECT_NAME} "${CBDESK_FFI_CRATE}" cbdesk_ffi "")

set(cbdesk_ffi_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  PARENT_SCOPE
)
