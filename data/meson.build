# Desktop file
desktop_file = i18n.merge_file(
  input: 'io.github.nacho.fastmath.desktop.in',
  output: 'io.github.nacho.fastmath.desktop',
  type: 'desktop',
  po_dir: meson.project_source_root() / 'po',
  install: true,
  install_dir: get_option('datadir') / 'applications',
)

# GSchema
install_data('io.github.nacho.fastmath.gschema.xml',
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas'
)

# Metainfo
metainfo_file = i18n.merge_file(
  input: 'io.github.nacho.fastmath.metainfo.xml',
  output: 'io.github.nacho.fastmath.metainfo.xml',
  po_dir: meson.project_source_root() / 'po',
  install: true,
  install_dir: get_option('datadir') / 'metainfo',
)

# Validation
desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test('validate-desktop', desktop_file_validate, args: [desktop_file])
endif

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('validate-metainfo', appstreamcli,
    args: ['validate', '--no-net', '--explain', metainfo_file])
endif

# Compile GSettings schemas
gnome.compile_schemas(build_by_default: true)

subdir('icons')
