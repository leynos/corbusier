import { renderApp } from '../../src/test/test-utils';
import { axe } from '../utils/axe';

describe('task route accessibility', () => {
  it('renders the task create route without axe violations', async () => {
    const { container } = await renderApp({ initialPath: '/tasks/new' });

    expect(await axe(container)).toHaveNoViolations();
  });

  it('renders the task detail route without axe violations', async () => {
    const { container } = await renderApp({
      initialPath: '/tasks/9f6adf0b-4908-47f5-a1fd-27d65f7d84bf',
    });

    expect(await axe(container)).toHaveNoViolations();
  });
});
