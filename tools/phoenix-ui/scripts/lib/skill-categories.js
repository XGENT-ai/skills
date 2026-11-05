/**
 * Category map for user-invocable skills, consumed by the provider
 * transformers.
 */

export const SKILL_CATEGORIES = {
  // CREATE - build something new
  'phoenix-ui': 'create',
  shape: 'create',
  // EVALUATE - review and assess
  critique: 'evaluate',
  audit: 'evaluate',
  // REFINE - improve existing design
  typeset: 'refine',
  layout: 'refine',
  colorize: 'refine',
  animate: 'refine',
  delight: 'refine',
  bolder: 'refine',
  quieter: 'refine',
  overdrive: 'refine',
  // SIMPLIFY - reduce and clarify
  distill: 'simplify',
  clarify: 'simplify',
  adapt: 'simplify',
  // HARDEN - production-ready
  polish: 'harden',
  optimize: 'harden',
  harden: 'harden',
  onboard: 'harden',
  // SYSTEM - setup and tooling
  init: 'system',
  document: 'system',
  extract: 'system',
  live: 'system',
  generate: 'system',
};

export const CATEGORY_ORDER = ['create', 'evaluate', 'refine', 'simplify', 'harden', 'system'];
