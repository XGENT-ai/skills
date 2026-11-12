/** Media argument and audit paths that do not contact a source. */
export default [
  { id: 'media-help', verb: 'media', args: ['--help'], offline: true },
  { id: 'media-unknown-option', verb: 'media', args: ['search', 'map', '--limt', '2'], offline: true },
  { id: 'media-missing-command', verb: 'media', args: [], offline: true },
  { id: 'media-search-missing-query', verb: 'media', args: ['search'], offline: true },
  {
    id: 'media-fetch-outside-area', verb: 'media', args: ['fetch', 'T1', '--dir', 'out'],
    offline: true, files: ['out/**', '.phoenix-ui/**'],
  },
  {
    id: 'media-audit-no-manifest', verb: 'media', args: ['audit', '--json'],
    offline: true, files: ['.phoenix-ui/**'],
  },
];
