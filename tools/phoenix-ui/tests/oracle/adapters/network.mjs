/** These static Next.js fixtures must not discover another project's port 3000. */
import { offlineNetworkProfile } from '../../../../../scripts/phoenix-build-tools.mjs';

export const isolatedCases = [
  'detect-fixture-text-framework-next-cssinjs',
  'detect-fixture-text-framework-next-modules',
  'detect-fixture-text-framework-next-tailwind',
  'detect-framework-next-modules-text',
];

export function nativeInvocation(id, argv, platform = process.platform) {
  return platform === 'darwin' && isolatedCases.includes(id)
    ? ['/usr/bin/sandbox-exec', '-p', offlineNetworkProfile, ...argv]
    : argv;
}

export { offlineNetworkProfile };
