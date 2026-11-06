/**
 * The installer verbs' static help (`install`, `link`, `update`, `check`).
 *
 * Only the help paths are pinned here: every other installer path writes into
 * harness directories or reaches the network, which the corpus keeps out. The
 * help text must render before any operational path runs (#708). `skills` is
 * not a command.
 */
export default [
  { id: 'skills-install-help', verb: 'install', args: ['--help'] },
  { id: 'skills-install-help-short', verb: 'install', args: ['-h'] },
  { id: 'skills-link-help', verb: 'link', args: ['--help'] },
  { id: 'skills-update-help', verb: 'update', args: ['--help'] },
  { id: 'skills-check-help', verb: 'check', args: ['--help'] },
  { id: 'skills-namespace-unknown', verb: 'skills', args: ['install', '--help'] },
];
