export const enGbMessages = {
  'app.title': 'Corbusier',
  'app.subtitle': 'Repository-owned task intake shell',
  'task.create.title': 'Create task from issue metadata',
  'task.create.description':
    'This fixture-backed slice mirrors the live task contract without coupling to backend auth or mutations yet.',
  'task.detail.title': 'Task detail',
  'task.detail.notFound': 'Task not found',
  'task.detail.notFoundBody':
    'No fixture task matched this identifier. The live transport seam lands in roadmap item 4.4.2.',
  'task.refs.branch.empty':
    'No branch linked yet. Live association lands in roadmap item 4.4.4.',
  'task.refs.pr.empty':
    'No pull request linked yet. Live association lands in roadmap item 4.4.4.',
  'task.form.errorBanner':
    'The fixture gateway rejected this submission. Adjust the input and retry.',
};

export type MessageKey = keyof typeof enGbMessages;
