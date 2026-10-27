/**
 * Unit tests for parsing Bun audit output at the command boundary.
 */
import { describe, expect, it } from 'vitest';

import {
  collectAdvisories,
  partitionAdvisoriesById,
} from '../../security/audit-utils.js';

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

  it('accepts direct package entries and empty audit output', () => {
    const directAdvisories = collectAdvisories({
      lodash: [{ github_advisory_id: 'GHSA-abcd-efgh-ijkl' }],
    });

    expect(directAdvisories).toMatchObject([
      {
        github_advisory_id: 'GHSA-abcd-efgh-ijkl',
        package: 'lodash',
      },
    ]);
    expect(collectAdvisories({})).toEqual([]);
  });

  it('rejects a malformed record rather than dropping valid advisories', () => {
    const mixed = {
      advisories: {
        lodash: [
          { github_advisory_id: 'GHSA-abcd-efgh-ijkl' },
          { github_advisory_id: 42 },
        ],
      },
    };

    expect(() => collectAdvisories(mixed)).toThrow(
      'Invalid bun audit advisory payload',
    );
  });

  it.each([
    { advisories: 'invalid' },
    { advisories: { lodash: 'invalid' } },
    { lodash: [{ title: 5 }] },
    { lodash: ['invalid'] },
  ])('rejects malformed advisory maps: %j', (input) => {
    expect(() => collectAdvisories(input)).toThrow(
      'Invalid bun audit advisory payload',
    );
  });

  it('prefers the explicit advisory ID and rejects unidentified records', () => {
    const advisories = collectAdvisories({
      lodash: [
        {
          github_advisory_id: 'GHSA-aaaa-bbbb-cccc',
          url: 'https://github.com/advisories/GHSA-dddd-eeee-ffff',
        },
        { url: 'https://example.com/advisory' },
      ],
    });

    const result = partitionAdvisoriesById(advisories, ['GHSA-aaaa-bbbb-cccc']);
    expect(result.expected).toMatchObject([
      { github_advisory_id: 'GHSA-aaaa-bbbb-cccc', package: 'lodash' },
    ]);
    expect(result.unexpected).toMatchObject([{ package: 'lodash' }]);
  });
});
