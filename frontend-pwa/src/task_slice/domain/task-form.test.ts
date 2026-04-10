/**
 * Unit tests for the task form helper functions.
 *
 * The suite covers `splitDelimitedValues`, `validateTaskCreateDraft`, and
 * `toCreateTaskRequest` so draft parsing stays stable for the route layer.
 */
import {
  splitDelimitedValues,
  toCreateTaskRequest,
  validateTaskCreateDraft,
} from './task-form';

describe('task form helpers', () => {
  it('validates required task create fields', () => {
    expect(
      validateTaskCreateDraft({
        provider: 'github',
        repository: 'invalid',
        issueNumber: '0',
        title: '   ',
        description: '',
        labels: '',
        assignees: '',
        milestone: '',
      }),
    ).toEqual({
      repository: 'Use the repository format owner/repository.',
      issueNumber: 'Issue number must be a positive integer.',
      title: 'Title is required.',
    });
  });

  it('normalizes optional lists and strings', () => {
    expect(splitDelimitedValues(' bug, p1, , backend ')).toEqual([
      'bug',
      'p1',
      'backend',
    ]);
    expect(
      toCreateTaskRequest({
        provider: 'github',
        repository: ' acme/widgets ',
        issueNumber: '42',
        title: ' Fix login flow ',
        description: ' Triage callback ',
        labels: 'bug, p1',
        assignees: 'alice',
        milestone: ' sprint-12 ',
      }),
    ).toEqual({
      provider: 'github',
      repository: 'acme/widgets',
      issue_number: 42,
      title: 'Fix login flow',
      description: 'Triage callback',
      labels: ['bug', 'p1'],
      assignees: ['alice'],
      milestone: 'sprint-12',
    });
  });

  it('rejects invalid issue numbers before building a request', () => {
    expect(() =>
      toCreateTaskRequest({
        provider: 'github',
        repository: 'acme/widgets',
        issueNumber: 'NaN',
        title: 'Fix login flow',
        description: '',
        labels: '',
        assignees: '',
        milestone: '',
      }),
    ).toThrow('Issue number must be a positive integer.');
  });
});
