import { defineConfig, devices } from '@playwright/test';
import path from 'path';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['json', { outputFile: 'test-results.json' }],
    process.env.CI ? ['github'] : ['list'],
  ],

  timeout: 30000,
  expect: {
    timeout: 10000,
  },

  use: {
    baseURL: process.env.TEST_BASE_URL || 'http://localhost:3001',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'on-first-retry',
    actionTimeout: 10000,
  },

  globalSetup: path.join(__dirname, 'fixtures/global-setup.ts'),
  globalTeardown: path.join(__dirname, 'fixtures/global-teardown.ts'),

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
    {
      name: 'mobile-chrome',
      use: { ...devices['Pixel 5'] },
    },
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 13'] },
    },
    {
      name: 'multiplayer',
      testDir: './e2e/multiplayer',
      use: { ...devices['Desktop Chrome'] },
      timeout: 60000, // Longer timeout for multi-player coordination
    },
  ],

  webServer: {
    command: 'cargo run --release -p bw-server',
    url: 'http://localhost:3001/health',
    cwd: path.join(__dirname, '..'),
    reuseExistingServer: !process.env.CI,
    timeout: 180000,
    env: {
      DATABASE_URL: 'sqlite:./testing/test.db',
      RUST_LOG: 'bw_server=warn',
      BW_PORT: '3001',
      BW_CONFIG: 'testing/config.toml',
    },
  },
});
