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

  # CocoaPods evaluates this file through
  # macos/Flutter/ephemeral/.symlinks. File.expand_path does not resolve
  # that link, so four `..` segments land in ephemeral/crates.
  here = File.realpath(__dir__)
  crate = File.expand_path('../../../../crates/cbdesk-ffi', here)
  cargo = File.join(crate, 'Cargo.toml')
  unless File.file?(cargo)
    raise "cbdesk_ffi: #{cargo} is missing (podspec #{__dir__} resolved to #{here})"
  end
  s.script_phase = {
    :name => 'Build Rust library',
    :script => "sh \"$PODS_TARGET_SRCROOT/../cargokit/build_pod.sh\" \"#{crate}\" cbdesk_ffi",
    :execution_position => :before_compile,
    :input_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony'],
    :output_files => ["${BUILT_PRODUCTS_DIR}/libcbdesk_ffi.a"],
  }
  # cargo links these when it builds a test binary. The static archive does
  # not carry them, so the pod link has to name them. CoreAudio and
  # AudioToolbox are cpal. CoreMedia is screen capture. SystemConfiguration
  # is the DNS resolver. Carbon is the keyboard layout used by input injection.
  s.frameworks = 'AudioToolbox', 'CoreAudio', 'CoreMedia', 'Carbon', 'SystemConfiguration'
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
    'OTHER_LDFLAGS' => '-force_load ${BUILT_PRODUCTS_DIR}/libcbdesk_ffi.a -framework AudioToolbox -framework CoreAudio -framework CoreMedia -framework Carbon -framework SystemConfiguration',
  }
end
