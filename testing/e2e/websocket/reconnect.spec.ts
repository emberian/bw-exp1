import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';
import { PageWebSocketHelper } from '../../fixtures/websocket';

test.describe('WebSocket Reconnection', () => {
  let auth: AuthHelper;

  test.beforeEach(async ({ page, baseURL }) => {
    auth = new AuthHelper(page, baseURL!);
  });

  test('should maintain game state while connected', async ({ page }) => {
    await auth.createAndLogin();
    await page.goto('/play/game');

    const ws = new PageWebSocketHelper(page);
    await ws.waitForConnection();
    await ws.waitForInitialState();

    // Get initial state
    const initialPosition = await page.locator('[data-testid="ship-position"]').textContent();

    // Wait a moment
    await page.waitForTimeout(2000);

    // State should still be present
    const currentPosition = await page.locator('[data-testid="ship-position"]').textContent();
    expect(currentPosition).toBeTruthy();
  });

  test('should show connection status indicator', async ({ page }) => {
    await auth.createAndLogin();
    await page.goto('/play/game');

    const ws = new PageWebSocketHelper(page);
    await ws.waitForConnection();

    // Should show "Connected" text somewhere
    await expect(page.locator('text=Connected')).toBeVisible({ timeout: 15000 });
  });

  test('should handle page refresh gracefully', async ({ page }) => {
    await auth.createAndLogin();
    await page.goto('/play/game');

    const ws = new PageWebSocketHelper(page);
    await ws.waitForConnection();
    await ws.waitForInitialState();

    // Refresh the page
    await page.reload();

    // Should reconnect and show game again
    const ws2 = new PageWebSocketHelper(page);
    await ws2.waitForConnection();
    await ws2.waitForInitialState();

    // Game should be functional
    await expect(page.locator('[data-testid="sector-map"]')).toBeVisible();
  });
});
