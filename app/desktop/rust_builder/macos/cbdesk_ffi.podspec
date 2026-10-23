Pod::Spec.new do |s|
  s.name             = 'cbdesk_ffi'
  s.version          = '0.0.1'
  s.summary          = 'CBDesk Rust core.'
  s.description      = 'Builds libcbdesk_ffi and links it into the desktop shell.'
  s.homepage         = 'https://github.com/cumabozkurt/cbdesk'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'CBDesk contributors' => 'cumabozkurt91@gmail.com' }

  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'FlutterMacOS'

  s.platform = :osx, '10.15'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
  s.swift_version = '5.0'

  crate = File.expand_path('../../../../crates/cbdesk-ffi', __dir__)
  s.script_phase = {
    :name => 'Build Rust library',
    :script => "sh \"$PODS_TARGET_SRCROOT/../cargokit/build_pod.sh\" \"#{crate}\" cbdesk_ffi",
    :execution_position => :before_compile,
    :input_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony'],
    :output_files => ["${BUILT_PRODUCTS_DIR}/libcbdesk_ffi.a"],
  }
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
    'OTHER_LDFLAGS' => '-force_load ${BUILT_PRODUCTS_DIR}/libcbdesk_ffi.a',
  }
end
