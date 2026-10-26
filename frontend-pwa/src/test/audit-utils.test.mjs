/**
 * Unit tests for parsing Bun audit output at the command boundary.
 */
import { describe, expect, it } from 'vitest';

import { collectAdvisories } from '../../security/audit-utils.js';

describe('collectAdvisories', () => {
  it('extracts advisory details from Bun nested output', () => {
    const advisories = collectAdvisories({
      advisories: {
        lodash: [
          {
            title: 'Prototype pollution',
            url: 'https://github.com/advisories/GHSA-abcd-efgh-ijkl',
          },
        ],
      },
    });

    expect(advisories).toMatchObject([
      {
        github_advisory_id: 'GHSA-abcd-efgh-ijkl',
        package: 'lodash',
        title: 'Prototype pollution',
      },
    ]);
  });

  it('accepts direct package entries and ignores malformed payloads', () => {
    const directAdvisories = collectAdvisories({
      lodash: [{ github_advisory_id: 'GHSA-abcd-efgh-ijkl' }],
    });

    expect(directAdvisories).toMatchObject([
      {
        github_advisory_id: 'GHSA-abcd-efgh-ijkl',
        package: 'lodash',
      },
    ]);
    expect(collectAdvisories({ advisories: 'invalid' })).toEqual([]);
  });
});
