import type { FullConfig } from '@playwright/test';
import { TestDatabase } from './database';

async function globalSetup(config: FullConfig) {
  console.log('Setting up test environment...');

  // Clean up test database (server will create fresh one on startup)
  const db = new TestDatabase();
  await db.setup();

  // Wait for server to be ready (Playwright's webServer config handles startup)
  const baseUrl = config.projects[0]?.use?.baseURL || 'http://localhost:3001';

  console.log(`Waiting for server at ${baseUrl}/health...`);

  const maxRetries = 60;
  for (let i = 0; i < maxRetries; i++) {
    try {
      const response = await fetch(`${baseUrl}/health`);
      if (response.ok) {
        console.log('Server is ready!');
        return;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  // If we get here, Playwright's webServer should have already started it
  // This is just a safety check
  console.log('Server health check completed (or webServer is handling startup)');
}

export default globalSetup;
