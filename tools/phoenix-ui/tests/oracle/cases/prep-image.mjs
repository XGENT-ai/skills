/** Image-processing arguments are validated before any input or output is opened. */
export default [
  { id: 'prep-image-help', verb: 'prep-image', args: ['--help'], offline: true },
  { id: 'prep-image-unknown-option', verb: 'prep-image', args: ['hero.png', '--widht', '100'], offline: true },
  { id: 'prep-image-missing-input', verb: 'prep-image', args: ['--width', '100'], offline: true },
  { id: 'prep-image-missing-width', verb: 'prep-image', args: ['hero.png', '--out', 'out'], offline: true, files: ['out/**'] },
];
