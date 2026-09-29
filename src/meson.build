pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
gnome = import('gnome')

gnome.compile_resources('eeman',
  'eeman.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
)

custom_target(
  'eeman',
  output: 'eeman',
  command: [
    'sh',
    meson.project_source_root() / 'src_rust' / 'build-eeman.sh',
    meson.project_source_root(),
    '@OUTPUT@',
    meson.project_build_root() / 'cargo-target',
  ],
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: get_option('bindir'),
  console: true,
)
