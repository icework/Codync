// Personal macOS distribution: production identity/data, locally managed updates.
const { readFileSync } = require('node:fs')
const { join } = require('node:path')
const { parse } = require('yaml')
const base = parse(readFileSync(join(__dirname, '../electron-builder.yml'), 'utf8'))
module.exports = {
  ...base,
  publish: null,
  mac: {
    ...base.mac,
    icon: 'resources/PersonalAppIcon.icon',
    target: ['dir'],
    timestamp: 'none',
    notarize: false,
  },
}
