import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('WebSocket Connection', () => {
  let auth: AuthHelper;

  test.beforeEach(async ({ page, baseURL }) => {
    auth = new AuthHelper(page, baseURL!);
  });

  test('should connect to WebSocket on game load', async ({ page }) => {
    await auth.createAndLogin();
    await page.goto('/play/game');

    const ws = new PageWebSocketHelper(page);

    // Should eventually show connected state
    await ws.waitForConnection();

    // Connection state should show "Connected"
    const state = await ws.getConnectionState();
    expect(state).toBe('Connected');
  });

  test('should receive initial state after connection', async ({ page }) => {
    await auth.createAndLogin();
    await page.goto('/play/game');

    const ws = new PageWebSocketHelper(page);
    await ws.waitForConnection();
    await ws.waitForInitialState();

    // Sector map should be visible with game state loaded
    await expect(page.locator('[data-testid="sector-map"]')).toBeVisible();

    // Ship status should be populated
    await expect(page.locator('[data-testid="ship-status"]')).toBeVisible();

    // Resource bar should be visible
    await expect(page.locator('[data-testid="resource-bar"]')).toBeVisible();
  });

  test('should populate player resources after connection', async ({ page }) => {
    await auth.createAndLogin();
    await page.goto('/play/game');

    const ws = new PageWebSocketHelper(page);
    await ws.waitForConnection();
    await ws.waitForInitialState();

    // Resource bar should show reputation
    const resourceBar = page.locator('[data-testid="resource-bar"]');
    await expect(resourceBar).toContainText('Rep');
  });

  test('should show ship position after connection', async ({ page }) => {
    await auth.createAndLogin();
    await page.goto('/play/game');

    const ws = new PageWebSocketHelper(page);
    await ws.waitForConnection();
    await ws.waitForInitialState();

    // Ship position should be displayed
    const position = page.locator('[data-testid="ship-position"]');
    await expect(position).toBeVisible();
    await expect(position).toContainText('X:');
  });
});
