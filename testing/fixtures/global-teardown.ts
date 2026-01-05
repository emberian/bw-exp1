import type { FullConfig } from '@playwright/test';

async function globalTeardown(config: FullConfig) {
  console.log('Tearing down test environment...');

  // Playwright's webServer config handles server shutdown automatically
  // This is here for any additional cleanup if needed

  console.log('Test environment teardown complete');
}

export default globalTeardown;
